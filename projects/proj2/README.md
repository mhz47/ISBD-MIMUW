# Projekt 2: Przygotowanie formatu pliku

Marta Hering-Zagrocka, 448197


---

Projekt implementuje format pliku kolumnowego (`.isbd`), który będzie służyć do składowania danych na dysku w analitycznym systemie DBMS. Format obsługuje trzy typy kolumn: INT64 (liczba całkowita 64-bitowa ze znakiem), BOOLEAN (wartość logiczna) oraz VARCHAR (napis o zmiennej długości), przy czym każdy typ wykorzystuje dedykowaną metodę kompresji dostosowaną do charakteru danych.

---

## Kompilacja i uruchamianie

### Struktura projektu

Projekt składa się z trzech niezależnych komponentów:
- biblioteka (`src/`): warstwa kodowania, formatu pliku oraz obliczania metryk.
- isbd-viewer (`apps/viewer.rs`): wczytuje plik ISBD i wyświetla metryki.
- isbd-generator (`apps/generator.rs`): generuje losowe pliki ISBD do testów.

### Kompilacja

W katalogu `projects/proj2/`:
- `make all`: kompiluje wszystkie komponenty (bibliotekę i oba programy).
- `make lib`: kompiluje wyłącznie bibliotekę.
- `make viewer`: kompiluje wyłącznie program `isbd-viewer`.
- `make generator`: kompiluje wyłącznie program `isbd-generator`.

### Uruchamianie

**Generowanie pliku testowego:**
```bash
./target/release/isbd-generator   # domyślnie: 10 wierszy -> data/sample.isbd
./target/release/isbd-generator --rows 500000 --output data/large.isbd
```

Program akceptuje dwa argumenty:

- `--rows N`: liczba wierszy do wygenerowania, domyślna wartość `10`,
- `--output PATH`: ścieżka do pliku wyjściowego, domyślna wartość `data/sample.isbd`.

Generator tworzy losowe dane i streamuje je batch po batchu do serializatora (batch po `PAGE_SIZE = 1024` wierszy, ostatni batch może być mniejszy). Symuluje to realistyczne wczytywanie danych.

- **INT64**: wartości „quasi-sekwencyjne" z małymi losowymi deltami (−10…+20), co dobrze testuje kompresję delta + VLE.


- **BOOLEAN**: losowe wartości `true`/`false`.


- **VARCHAR**: losowe napisy ASCII o zmiennej długości (3–12 znaków dla kolumny „name", 10–50 znaków dla „description").

**Wczytywanie pliku i wyświetlanie metryk:**
```bash
./target/release/isbd-viewer data/sample.isbd
```

Program wczytuje plik używając strumieniowego deserializatora, co pozwala przetwarzać pliki większe niż dostępna pamięć RAM. Metryki są obliczane batch po batchu bez ładowania całego pliku do pamięci.

**Przykład wyjścia**:
```
Loading file: data/sample.isbd

--- INT64 Metrics ---
Column 'id': average = 682.8000

--- BOOLEAN Metrics ---
Column 'active': binary entropy = 0.8813

--- VARCHAR Metrics ---
Column 'name': character count = 72
Column 'description': character count = 324
```

---

## Format pliku ISBD i implementacja

### Format
Format pliku ISBD to własny binarny format kolumnowy inspirowany Apache Parquet. Struktura pliku:

```
+------------------+
|  Magic "ISBD1"   |  (5 bajtów, identyfikacja formatu)
+------------------+
|  Batch 0, Kol 0  |  (niezależnie skompresowany, do 1024 wierszy)
|  Batch 0, Kol 1  |  
|  ...             |
|  Batch 1, Kol 0  |  
|  ...             |
+------------------+
|  Stopka (Footer) |  (num_rows, num_columns, schematy kolumn z offsetami + liczbą wierszy per page)
+------------------+
|  Długość stopki  |  (8 bajtów, u64 LE - rozmiar metadanych stopki powyżej)
+------------------+
|  Magic "1DBSI"   |  (5 bajtów, odwrócone, weryfikacja integralności)
+------------------+
```

Dane są zapisywane batch po batchu (wszystkie kolumny batcha 0, następnie batch 1 itd.), co umożliwia strumieniowy serializator pracujący na iteratorze - każdy batch jest kodowany i zapisywany natychmiast. Kolumny są dzielone na fragmenty po `PAGE_SIZE = 1024` wierszy, każdy skompresowany niezależnie. Stopka przechowuje offsety oraz liczbę wierszy per page, pozwalając deserializatorowi użyć `seek()` do bezpośredniego dostępu do dowolnego page'a bez ładowania całej kolumny. Magic bytes `ISBD1`/`1DBSI` na początku i końcu pliku weryfikują integralność danych.

### Kompresja w zależności od typu kolumny

#### Delta encoding + VLE dla INT64

Dla kolumn INT64 zastosowano trójetapową kompresję na poziomie page'a:

1. **Delta encoding**: zamiast przechowywać wartości $v_0, v_1, \dots, v_n$, przechowuje się różnice $\Delta_i = v_i - v_{i-1}$ (dla $i=0$: $\Delta_0 = v_0$). Dla danych posortowanych lub wolno rosnących (np. identyfikatory, timestampy) delty są małe. Delta jest resetowana na początku każdego page'a - pierwszy wiersz przechowuje pełną wartość, co gwarantuje niezależność page'ów.


2. **ZigZag encoding**: mapuje liczby ze znakiem na bezznakowe.
    Wzór: $(x \ll 1) \oplus (x \gg 63)$. Pozwala to kodować małe liczby ujemne tak samo krótko jak małe dodatnie.


3. **Variable Length Encoding (varint)**: koduje liczbę bezznakową używając 7 bitów na bajt, z 8. bitem jako flagą kontynuacji.

#### Bit-packing dla BOOLEAN

Każda wartość boolean zajmuje dokładnie 1 bit (true = 1, false = 0), pakowany w bajty LSB first. Ostatni bajt może zawierać niewykorzystane bity (padding). Osiąga to 8-krotną redukcję względem standardowego `Vec<bool>`.

#### ZSTD dla VARCHAR

Napis VARCHAR jest kodowany jako: długość (varint) + dane (bajty UTF-8). Każdy page jest następnie niezależnie kompresowany algorytmem ZSTD, który oferuje wysoki stopień skrócenia przy szybkiej dekompresji.

### Reprezentacja w pamięci

Dane w pamięci są przechowywane w formacie SoA, co optymalizuje lokalność cache i umożliwia operacje SIMD. Deserializator używa strumieniowego odczytu pliku i wczytuje tylko page potrzebny dla danego batcha. Liczba wierszy per page jest odczytywana z tablicy `page_row_counts` w stopce, więc deserializator wie dokładnie, ile wartości zdekodować (kluczowe dla bit-packingu BOOLEAN, gdzie liczba bitów musi zgadzać się z liczbą wierszy). Dla kolumn BOOLEAN odczytywane są tylko potrzebne bity z page'a, dla INT64 i VARCHAR dekodowany jest jeden page na raz. Oznacza to, że żadna kolumna nie jest w całości ładowana do pamięci, a w RAM znajduje się tylko jeden page.

### Semantyka batchy i page'ów

Serializator akceptuje batche o dowolnym rozmiarze. Zależność między batchami wejściowymi a page'ami na dysku zależy od rozmiaru batcha:

| Rozmiar batcha | Serializacja | Deserializacja |
|----------------|--------------|----------------|
| **≤ PAGE_SIZE**  | Batch staje się jednym page'em na kolumnę | Zwraca jeden batch na page - początkowa struktura zachowana 1:1 |
| **> PAGE_SIZE**  | Batch dzieli się na wiele page'ów po maksymalnie 1024 wiersze | Zwraca wiele batchy (po jednym na page) - granice batchy się zmieniają |

Niezależnie od rozmiaru batcha, dane są zawsze bitowo identyczne po roundtripie. Różnicą jest to, że batch większy niż `PAGE_SIZE` jest dzielony na wiele page'ów na dysku, a deserializator zwraca po jednym batchu na page zamiast jednego batchu na oryginalny input.

### Wymagania pamięciowe

Zużycie RAM nie rośnie wraz z rozmiarem pliku. Zależy ono od trzech czynników:

1. **Liczba kolumn** - każda kolumna wymaga jednego page'a danych w pamięci na raz. Więcej kolumn oznacza proporcjonalnie więcej RAM.


2. **Długość napisów VARCHAR** - czynnik zmienny. Rozmiar page'a VARCHAR zależy od całkowitej długości napisów we wszystkich wierszach - dla typowych danych (imiona, opisy itp.) jest to kilka do kilkuset KB.


3. **Rozmiar page'a** - maksymalnie `PAGE_SIZE = 1024` wierszy. Serializator i deserializator przetwarzają jeden page na raz, więc pamięć jest ograniczona rozmiarem jednego page'a na kolumnę.

W praktyce oznacza to, że plik z milionami wierszy i dziesiątkami kolumn można przetworzyć na maszynie z kilkuset MB RAM (o ile pojedyncze napisy VARCHAR nie są ekstremalnie długie).

### Maksymalne rozmiary danych w polach

Format obsługuje bardzo duże pliki:

- **INT64** - pełny zakres i64 (−2⁶³ … 2⁶³−1), kodowany jako delta + ZigZag + varint (do 9 bajtów na wartość),
- **BOOLEAN** - 1 bit na wartość,
- **VARCHAR** - każdy napis może mieć do ~9.2 EB (2⁶³−1 bajtów), kodowany jako długość (varint) + bajty UTF-8, kompresja ZSTD per page (teoretycznie page z samymi tak długimi napisami przekroczyłby możliwy zakres rozmiaru page'a w stopce (u64), ale w praktyce jest to niemożliwe ze względu ograniczenia sprzętowe),
- **liczba wierszy** - do 2⁶⁴−1, zapisane jako u64 w stopce,
- **liczba kolumn** - bez sztywnego limitu (ochrona przed przekroczeniem pamięci: limit rozmiaru stopki do 64 MB)*,
- **rozmiar pliku** - do ~16 EB (2⁶⁴−1 bajtów), ograniczony offsetami u64 page'ów.

Praktycznym limitem dla większości użycia jest przestrzeń dyskowa i czas przetwarzania, a nie pamięć. Strumieniowa architektura gwarantuje, że zużycie RAM pozostaje proporcjonalne do liczby kolumn i rozmiaru page'a, niezależnie od liczby wierszy w pliku.

<small>\* Limit rozmiaru stopki chroni przed atakami DoS i przekroczeniem pamięci: złośliwy plik z milionem kolumn lub stron spowodowałby błąd przy deserializacji, zanim zostanie zaalokowana duża ilość pamięci. W praktyce stopka rośnie proporcjonalnie do liczby kolumn × liczby page'ów.</small>

### Bezpieczeństwo transakcyjne

Serializator najpierw zapisuje dane do pliku tymczasowego (`.tmp`), a dopiero po pomyślnym zapisie wszystkich danych na dysk wykonuje `std::fs::rename()`. Ta atomowa operacja POSIX gwarantuje, że jeśli proces upadnie w trakcie zapisu, ostateczny plik `.isbd` pozostaje nienaruszony - na dysku znajduje się albo stary, poprawny plik, albo nic (nigdy uszkodzony plik częściowy).


### Metryki

Program `isbd-viewer` oblicza metryki dla każdego typu kolumny:

**INT64 - Średnia arytmetyczna**

$$\bar{x} = \frac{1}{n}\sum_{i=1}^{n} x_i$$

**BOOLEAN - Entropia binarna**

$$H(p) = -p \cdot \log_2(p) - (1-p) \cdot \log_2(1-p)$$

gdzie $p$ to ułamek wartości `true`.

**VARCHAR - Liczba znaków ASCII**

$$\sum_{i=1}^{n} \text{chars}(s_i)$$

Liczba znaków ASCII we wszystkich napisach.



Metryki są obliczane inkrementalnie - jeden batch na raz, utrzymując bieżące sumy i liczniki bez przechowywania wszystkich wartości w pamięci.

---

## Testy


```bash
make test              # wszystkie testy
make test-unit         # tylko testy jednostkowe
make test-integration  # tylko testy integracyjne
```

**Testy jednostkowe** weryfikują poprawność każdego komponentu kodowania osobno: VLE (varint), delta encoding, bit-packing BOOLEAN, kompresja ZSTD dla VARCHAR, serializacja stopki, obliczanie metryk oraz dostęp na poziomie wiersza (get/set).

**Testy integracyjne** sprawdzają pełne przepływy przez serializację i deserializację: roundtrip z weryfikacją tożsamości danych (małe i duże batche, zmienne rozmiary), przypadki brzegowe (puste stringi, ekstremalne wartości INT64), obsługę błędów (uszkodzony header, obcięte pliki, puste dane), niezależność page'ów (seek do dowolnego page'a bez wczytywania poprzednich) oraz metryki strumieniowe. Dodatkowo mamy roundtrip pod ograniczoną pamięcią.
Test używa `ulimit -v`** do ograniczenia przestrzeni adresowej procesu do 256 MB, generuje plik z 20_000_000 wierszy, deserializuje go i porównuje metryki z wynikiem bez limitu. 

Testy roundtrip teoretycznie mogą ukryć błędy symetryczne, jeśli ten sam błąd występuje zarówno w serializatorze, jak i w deserializatorze. Ryzyko to jest zminimalizowane dzięki testom jednostkowym, które weryfikują surowe bajty kodowania oraz testują funkcje kodowania niezależnie od dekodowania. Dodatkowo test page independence weryfikuje offsety zapisane w stopce - seek do konkretnego page'a bez wczytywania poprzednich wykryłby błędy w kolejności lub wartościach offsetów, nawet gdyby serializator i deserializator zawierały ten sam symetryczny błąd.

<small>\*\* `ulimit -v` jest kompromisem na potrzeby testowania przetwarzania danych większych niż dostępna pamięć RAM. `systemd-run --scope -p MemoryMax` ogranicza fizyczną pamięć za pomocą cgroups v2, ale wymaga uprawnień administratora, co komplikuje automatyczne testy. `ulimit -v` ogranicza wirtualną przestrzeń adresową procesu. Jest mniej precyzyjne, ale wystarczające, bo przy próbie wczytania wszystkich wierszy do pamięci na raz, przestrzeń adresowa i tak przekroczyłaby limit.</small>
