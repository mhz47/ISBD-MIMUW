## Kompilacja i uruchamianie

Projekt można skompilować za pomocą dostarczonego pliku `Makefile`:

```bash
make all
```

Skompilowane pliki binarne trafią do `src/target/release/`:
* `latency` – wielowątkowy losowy odczyt (`pread`),
* `throughput` – sekwencyjny odczyt z mapowaniem pamięci (`mmap`),
* `pipeline` – pipeline producent-konsument (`sync_channel`),
* `latency_original`, `throughput_original`, `pipeline_original` – wersje oryginalne (przed modyfikacjami).

### Uruchomienie pojedynczych programów

```bash
# 1. Latencja (losowy odczyt)
./src/target/release/latency <ścieżka> [rozmiar_bloku] [wątki]

# 2. Przepustowość (zero-copy mmap)
./src/target/release/throughput <ścieżka> [rozmiar_bloku]

# 3. Pipeline I/O i obliczeń
./src/target/release/pipeline <ścieżka> [rozmiar_bloku] [iteracje]
```

## Benchmarki i generowanie wykresów

Aby uruchomić pełny zestaw testów na trzech rozmiarach plików (10, 50, 200 MB):

```bash
make benchmark
```

Aby wygenerować wykresy z istniejących wyników (bez ponownego uruchamiania testów):

```bash
make plots
```

Skrypt `plot_all.py` czyta pliki CSV z `benchmark/results/` i generuje osobne wykresy dla każdego rozmiaru pliku w `benchmark/plots/`.

Wymagania dla skryptów testowych:
* Python 3
* `matplotlib` (`pip install matplotlib`)

Wyniki pomiarów zostaną zapisane w `benchmark/results/`, a wykresy w `benchmark/plots/`.
