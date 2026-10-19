# Czyta wyniki z plików CSV i generuje wykresy.
# Użycie: python3 plot_all.py [rozmiar_mb]
#   rozmiar_mb: opcjonalny filtr, np. 50 — generuje tylko wykresy dla pliku 50 MB
#   bez argumentu: generuje wykresy dla wszystkich rozmiarów plików

import csv
import sys
from pathlib import Path
import matplotlib.pyplot as plt


BENCH_DIR = Path(__file__).resolve().parent
RESULTS_DIR = BENCH_DIR / "results"
PLOTS_DIR = BENCH_DIR / "plots"


def format_bytes(n):
    n = int(n)
    if n < 1024: return f"{n} B"
    if n < 1024 * 1024: return f"{n // 1024} KB"
    return f"{n // (1024 * 1024)} MB"

def load_csv(name):
    path = RESULTS_DIR / name
    if not path.exists():
        print(f"  [!] Brak pliku {path}, pomijam.")
        return []
    with open(path, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))

m_colors = {
    512: "#e63946",
    1024: "#2a9d8f",
    4096: "#264653",
    16384: "#f4a261",
    65536: "#9b5de5",
    1048576: "#00b894",
    4194304: "#fd79a8",
    16777216: "#636e72",
}

def get_color(m):
    return m_colors.get(int(m), "#6c5ce7")

def plot_latency(data, fs_filter=None):
    """Wykresy latencji: czas całkowity i średni czas odczytu w funkcji wątków.
    2 panele (warm/cold), każdy z wieloma liniami — po jednej na rozmiar bloku M."""
    if fs_filter:
        data = [r for r in data if float(r.get("file_size_mb", 50)) == fs_filter]
    if not data:
        return

    file_sizes = sorted(list(set(float(r.get("file_size_mb", 50)) for r in data)))
    all_threads = sorted(list(set(int(r["threads"]) for r in data if r["version"] == "improved")))

    for fs in file_sizes:
        fs_data = [r for r in data if float(r.get("file_size_mb", 50)) == fs]
        fs_suffix = f"_fs{int(fs)}" if len(file_sizes) > 1 else ""
        m_list = sorted(list(set(int(r["m_bytes"]) for r in fs_data)))

        # czas całkowity vs wątki
        fig, axes = plt.subplots(1, 2, figsize=(14, 5))
        for ax_idx, (cache_name, cache_title) in enumerate([("warm", "Warm Cache"), ("cold", "Cold Cache")]):
            ax = axes[ax_idx]
            cdata = [r for r in fs_data if r["cache"] == cache_name]
            if not cdata:
                continue

            for m in m_list:
                m_label = format_bytes(m)
                color = get_color(m)

                # improved
                opt = sorted([r for r in cdata if int(r["m_bytes"]) == m and r["version"] == "improved"],
                             key=lambda x: int(x["threads"]))
                if opt:
                    ax.plot([int(r["threads"]) for r in opt], [float(r["time_ms"]) for r in opt],
                            marker="o", linewidth=2, color=color, markersize=5, label=f"M={m_label} (improved)")

                # original 
                orig = next((r for r in cdata if int(r["m_bytes"]) == m and r["version"] == "original"), None)
                if orig:
                    ax.hlines(float(orig["time_ms"]), min(all_threads), max(all_threads),
                              colors=color, linestyles="--", linewidth=1.2, alpha=0.5, label=f"M={m_label} (original)")

            ax.set_title(cache_title, fontsize=11, fontweight="bold")
            ax.set_xlabel("Wątki (T)", fontsize=10)
            ax.set_ylabel("Czas [ms]", fontsize=10)
            ax.set_xticks(all_threads)
            ax.legend(fontsize=8, loc="upper right")
            ax.grid(True, linestyle="--", alpha=0.4)

        fig.suptitle(f"Czas losowego odczytu (plik {int(fs)} MB)", fontsize=12, fontweight="bold")
        plt.tight_layout(rect=[0, 0, 0.97, 0.95])
        plt.savefig(PLOTS_DIR / f"latency_time_vs_threads{fs_suffix}.png", dpi=200, bbox_inches="tight")
        plt.close()

        # sredni czas odczytu vs wątki
        fig, axes = plt.subplots(1, 2, figsize=(14, 5))
        for ax_idx, (cache_name, cache_title) in enumerate([("warm", "Warm Cache"), ("cold", "Cold Cache")]):
            ax = axes[ax_idx]
            cdata = [r for r in fs_data if r["cache"] == cache_name]
            if not cdata:
                continue

            for m in m_list:
                m_label = format_bytes(m)
                color = get_color(m)

                opt = sorted([r for r in cdata if int(r["m_bytes"]) == m and r["version"] == "improved"],
                             key=lambda x: int(x["threads"]))
                if opt:
                    ax.plot([int(r["threads"]) for r in opt], [float(r["śr_odczyt_µs"]) for r in opt],
                            marker="o", linewidth=2, color=color, markersize=5, label=f"M={m_label} (improved)")

                orig = next((r for r in cdata if int(r["m_bytes"]) == m and r["version"] == "original"), None)
                if orig:
                    ax.hlines(float(orig["śr_odczyt_µs"]), min(all_threads), max(all_threads),
                              colors=color, linestyles="--", linewidth=1.2, alpha=0.5, label=f"M={m_label} (original)")

            ax.set_title(cache_title, fontsize=11, fontweight="bold")
            ax.set_xlabel("Wątki (T)", fontsize=10)
            ax.set_ylabel("Śr. czas odczytu [µs]", fontsize=10)
            ax.set_xticks(all_threads)
            ax.legend(fontsize=8, loc="upper right")
            ax.grid(True, linestyle="--", alpha=0.4)

        fig.suptitle(f"Średni czas pread() (plik {int(fs)} MB)", fontsize=12, fontweight="bold")
        plt.tight_layout(rect=[0, 0, 0.97, 0.95])
        plt.savefig(PLOTS_DIR / f"latency_read_vs_threads{fs_suffix}.png", dpi=200, bbox_inches="tight")
        plt.close()

        print(f"  [+] Latency wykresy ({int(fs)} MB): time_vs_threads, read_vs_threads")


def plot_throughput(data, fs_filter=None):
    """Wykresy throughput: mmap vs read w funkcji M.
    2 panele (warm/cold), 2 linie (read/mmap) — prosty i czytelny."""
    if fs_filter:
        data = [r for r in data if float(r.get("file_size_mb", 50)) == fs_filter]
    if not data:
        return

    file_sizes = sorted(list(set(float(r.get("file_size_mb", 50)) for r in data)))

    for fs in file_sizes:
        fs_data = [r for r in data if float(r.get("file_size_mb", 50)) == fs]
        fs_suffix = f"_fs{int(fs)}" if len(file_sizes) > 1 else ""

        # czas całkowity mmap vs read
        fig, axes = plt.subplots(1, 2, figsize=(12, 5))
        for ax_idx, (cache_name, title) in enumerate([("warm", "Warm Cache"), ("cold", "Cold Cache")]):
            ax = axes[ax_idx]
            cdata = [r for r in fs_data if r["cache"] == cache_name]
            if not cdata:
                continue
            m_list = sorted(list(set(int(r["m_bytes"]) for r in cdata)))
            labels = [format_bytes(m) for m in m_list]

            t_mmap = [next(float(r["time_ms"]) for r in cdata if int(r["m_bytes"]) == m and r["method"] == "mmap") for m in m_list]
            t_read = [next(float(r["time_ms"]) for r in cdata if int(r["m_bytes"]) == m and r["method"] == "read") for m in m_list]

            ax.plot(range(len(m_list)), t_read, marker="s", linewidth=2, linestyle="--",
                    color="#e63946", label="read()")
            ax.plot(range(len(m_list)), t_mmap, marker="o", linewidth=2,
                    color="#2a9d8f", label="mmap()")

            ax.set_title(title, fontsize=11, fontweight="bold")
            ax.set_xlabel("Rozmiar bloku M", fontsize=10)
            ax.set_ylabel("Czas [ms]", fontsize=10)
            ax.set_xticks(range(len(m_list)))
            ax.set_xticklabels(labels, rotation=35)
            ax.legend(fontsize=9)
            ax.grid(True, linestyle="--", alpha=0.4)

        fig.suptitle(f"Przepustowość: read() vs mmap() (plik {int(fs)} MB)", fontsize=12, fontweight="bold")
        plt.tight_layout()
        plt.savefig(PLOTS_DIR / f"throughput_mmap_vs_read{fs_suffix}.png", dpi=200)
        plt.close()

        # czas jądra (sys time)
        fig, axes = plt.subplots(1, 2, figsize=(12, 5))
        for ax_idx, (cache_name, title) in enumerate([("warm", "Warm Cache"), ("cold", "Cold Cache")]):
            ax = axes[ax_idx]
            cdata = [r for r in fs_data if r["cache"] == cache_name]
            if not cdata:
                continue
            m_list = sorted(list(set(int(r["m_bytes"]) for r in cdata)))
            labels = [format_bytes(m) for m in m_list]

            mmap_sorted = sorted([(int(r["m_bytes"]), float(r["stime_s"]) * 1000) for r in cdata if r["method"] == "mmap"])
            read_sorted = sorted([(int(r["m_bytes"]), float(r["stime_s"]) * 1000) for r in cdata if r["method"] == "read"])

            ax.plot(range(len(m_list)), [v for _, v in read_sorted], marker="s", linewidth=2,
                    linestyle="--", color="#e63946", label="read()")
            ax.plot(range(len(m_list)), [v for _, v in mmap_sorted], marker="o", linewidth=2,
                    color="#2a9d8f", label="mmap()")

            ax.set_title(title, fontsize=11, fontweight="bold")
            ax.set_xlabel("Rozmiar bloku M", fontsize=10)
            ax.set_ylabel("Czas jądra [ms]", fontsize=10)
            ax.set_xticks(range(len(m_list)))
            ax.set_xticklabels(labels, rotation=35)
            ax.legend(fontsize=9)
            ax.grid(True, linestyle="--", alpha=0.4)

        fig.suptitle(f"Narzut wywołań systemowych (plik {int(fs)} MB)", fontsize=12, fontweight="bold")
        plt.tight_layout()
        plt.savefig(PLOTS_DIR / f"throughput_sys_time{fs_suffix}.png", dpi=200)
        plt.close()

        print(f"  [+] Throughput wykresy ({int(fs)} MB): mmap_vs_read, sys_time")


def plot_pipeline(data, fs_filter=None):
    """Wykresy pipeline: czas w funkcji K i dekompozycja."""
    if fs_filter:
        data = [r for r in data if float(r.get("file_size_mb", 50)) == fs_filter]
    if not data:
        return

    file_sizes = sorted(list(set(float(r.get("file_size_mb", 50)) for r in data)))

    for fs in file_sizes:
        fs_data = [r for r in data if float(r.get("file_size_mb", 50)) == fs]
        fs_suffix = f"_fs{int(fs)}" if len(file_sizes) > 1 else ""

        # czas w funkcji K (k_opt)
        k_data = [r for r in fs_data if r["test"] == "k_opt"]
        if k_data:
            fig, axes = plt.subplots(1, 2, figsize=(12, 5))
            for ax_idx, (cache_name, title) in enumerate([("warm", "Warm Cache"), ("cold", "Cold Cache")]):
                ax = axes[ax_idx]
                cdata = [r for r in k_data if r["cache"] == cache_name]
                if not cdata:
                    continue
                k_list = sorted(list(set(int(r["k"]) for r in cdata)))

                t_pipe = [next(float(r["time_ms"]) for r in cdata if int(r["k"]) == k and r["method"] == "pipeline") for k in k_list]
                t_orig = [next(float(r["time_ms"]) for r in cdata if int(r["k"]) == k and r["method"] == "original") for k in k_list]

                ax.plot(k_list, t_orig, marker="s", linewidth=2, linestyle="--",
                        color="#e63946", label="Sekwencyjny")
                ax.plot(k_list, t_pipe, marker="o", linewidth=2,
                        color="#2a9d8f", label="Pipeline")

                ax.set_title(title, fontsize=11, fontweight="bold")
                ax.set_xlabel("Iteracje (K)", fontsize=10)
                ax.set_ylabel("Czas [ms]", fontsize=10)
                ax.legend(fontsize=9)
                ax.grid(True, linestyle="--", alpha=0.4)

            fig.suptitle(f"Pipeline vs sekwencyjny w funkcji K (plik {int(fs)} MB)", fontsize=12, fontweight="bold")
            plt.tight_layout()
            plt.savefig(PLOTS_DIR / f"pipeline_k_opt{fs_suffix}.png", dpi=200)
            plt.close()

            # dekompozycja pipeline
            fig, axes = plt.subplots(1, 2, figsize=(12, 5))
            for ax_idx, (cache_name, title) in enumerate([("warm", "Warm Cache"), ("cold", "Cold Cache")]):
                ax = axes[ax_idx]
                cdata = [r for r in k_data if r["cache"] == cache_name and r["method"] == "pipeline"]
                if not cdata:
                    continue
                k_list = sorted(list(set(int(r["k"]) for r in cdata)))

                t_total = [next(float(r["time_ms"]) for r in cdata if int(r["k"]) == k) for k in k_list]
                t_read = [next(float(r["czas_odczyt_s"]) * 1000 for r in cdata if int(r["k"]) == k) for k in k_list]
                t_compute = [next(float(r["czas_oblicz_s"]) * 1000 for r in cdata if int(r["k"]) == k) for k in k_list]

                ax.plot(k_list, t_total, marker="o", linewidth=2, color="#2a9d8f", label="T_total")
                ax.plot(k_list, t_read, marker="x", linewidth=1.5, linestyle=":", color="#f4a261", label="T_read")
                ax.plot(k_list, t_compute, marker="^", linewidth=1.5, linestyle="--", color="#9b5de5", label="T_compute")

                ax.set_title(title, fontsize=11, fontweight="bold")
                ax.set_xlabel("Iteracje (K)", fontsize=10)
                ax.set_ylabel("Czas [ms]", fontsize=10)
                ax.legend(fontsize=9)
                ax.grid(True, linestyle="--", alpha=0.4)

            fig.suptitle(f"Dekompozycja czasu pipeline (plik {int(fs)} MB)", fontsize=12, fontweight="bold")
            plt.tight_layout()
            plt.savefig(PLOTS_DIR / f"pipeline_breakdown{fs_suffix}.png", dpi=200)
            plt.close()

            print(f"  [+] Pipeline wykresy ({int(fs)} MB): k_opt, breakdown")

        # czas w funkcji M dla różnych K
        m_data = [r for r in fs_data if r["test"] == "m_sens"]
        if m_data:
            k_list = sorted(list(set(int(r["k"]) for r in m_data)))
            m_list = sorted(list(set(int(r["m_bytes"]) for r in m_data)))
            labels = [format_bytes(m) for m in m_list]
            n_k = len(k_list)

            fig, axes = plt.subplots(1, n_k, figsize=(4 * n_k, 4))
            if n_k == 1:
                axes = [axes]

            for k_idx, k in enumerate(k_list):
                ax = axes[k_idx]
                for method, style, marker, color in [
                    ("pipeline", "-", "o", "#2a9d8f"),
                    ("original", "--", "s", "#e63946"),
                ]:
                    times = []
                    for m in m_list:
                        r = next((x for x in m_data if int(x["m_bytes"]) == m and int(x["k"]) == k and x["method"] == method), None)
                        times.append(float(r["time_ms"]) if r else 0)
                    ax.plot(range(len(m_list)), times, marker=marker, linestyle=style,
                            linewidth=2, color=color, label=method)

                ax.set_title(f"K = {k}", fontsize=10, fontweight="bold")
                ax.set_xlabel("Rozmiar bloku M", fontsize=9)
                ax.set_ylabel("Czas [ms]", fontsize=9)
                ax.set_xticks(range(len(m_list)))
                ax.set_xticklabels(labels, rotation=35)
                if k_idx == 0:
                    ax.legend(fontsize=9)
                ax.grid(True, linestyle="--", alpha=0.4)

            fig.suptitle(f"Pipeline w funkcji M (Warm Cache, plik {int(fs)} MB)", fontsize=12, fontweight="bold")
            plt.tight_layout()
            plt.savefig(PLOTS_DIR / f"pipeline_vs_m{fs_suffix}.png", dpi=200)
            plt.close()

            print(f"  [+] Pipeline wykres ({int(fs)} MB): vs_m")


def plot_memory(data):
    """Wykresy presji pamięciowej — wszystkie M na jednym wykresie z legendą."""
    if not data:
        return

    # latency
    lat_data = [r for r in data if r.get("program") == "latency"]
    if lat_data:
        m_list = sorted(list(set(int(r["m_bytes"]) for r in lat_data)))
        all_threads = sorted(list(set(int(r["threads"]) for r in lat_data if r["version"] == "improved")))

        fig, ax = plt.subplots(figsize=(10, 5))

        for m in m_list:
            m_label = format_bytes(m)
            color = get_color(m)

            improved = sorted([r for r in lat_data if int(r["m_bytes"]) == m and r["version"] == "improved"],
                              key=lambda x: int(x["threads"]))
            if improved:
                ax.plot([int(r["threads"]) for r in improved], [float(r["time_ms"]) for r in improved],
                        marker="o", linewidth=2, color=color, markersize=5, label=f"M={m_label} (improved)")

            orig = next((r for r in lat_data if int(r["m_bytes"]) == m and r["version"] == "original"), None)
            if orig:
                ax.hlines(float(orig["time_ms"]), min(all_threads), max(all_threads),
                          colors=color, linestyles="--", linewidth=1.2, alpha=0.5, label=f"M={m_label} (original)")

        ax.set_title("Losowy odczyt przy presji pamięci (MemoryMax=16M)", fontsize=11, fontweight="bold")
        ax.set_xlabel("Wątki (T)", fontsize=10)
        ax.set_ylabel("Czas [ms]", fontsize=10)
        ax.set_xticks(all_threads)
        ax.legend(fontsize=9, loc="upper right")
        ax.grid(True, linestyle="--", alpha=0.4)

        plt.tight_layout()
        plt.savefig(PLOTS_DIR / "memory_pressure_latency.png", dpi=200, bbox_inches="tight")
        plt.close()
        print("  [+] Memory pressure: latency")

    # throughput
    tp_data = [r for r in data if r.get("program") == "throughput"]
    if tp_data:
        fig, ax = plt.subplots(figsize=(8, 5))
        m_list = sorted(list(set(int(r["m_bytes"]) for r in tp_data)))
        labels = [format_bytes(m) for m in m_list]

        mmap_sorted = sorted([(int(r["m_bytes"]), float(r["time_ms"])) for r in tp_data if r["version"] == "mmap"])
        read_sorted = sorted([(int(r["m_bytes"]), float(r["time_ms"])) for r in tp_data if r["version"] == "read"])

        ax.plot(range(len(m_list)), [v for _, v in read_sorted], marker="s", linewidth=2,
                linestyle="--", color="#e63946", label="read()")
        ax.plot(range(len(m_list)), [v for _, v in mmap_sorted], marker="o", linewidth=2,
                color="#2a9d8f", label="mmap()")

        ax.set_title("Przepustowość przy presji pamięci (MemoryMax=16M)", fontsize=11, fontweight="bold")
        ax.set_xlabel("Rozmiar bloku M", fontsize=10)
        ax.set_ylabel("Czas [ms]", fontsize=10)
        ax.set_xticks(range(len(m_list)))
        ax.set_xticklabels(labels, rotation=35)
        ax.legend(fontsize=9)
        ax.grid(True, linestyle="--", alpha=0.4)
        plt.tight_layout()
        plt.savefig(PLOTS_DIR / "memory_pressure_throughput.png", dpi=200)
        plt.close()
        print("  [+] Memory pressure: throughput")


def main():
    PLOTS_DIR.mkdir(parents=True, exist_ok=True)

    fs_filter = None
    if len(sys.argv) > 1:
        try:
            fs_filter = float(sys.argv[1])
            print(f"Generowanie wykresów tylko dla plików {fs_filter} MB...")
        except ValueError:
            print(f"Użycie: python3 plot_all.py [rozmiar_mb]")
            sys.exit(1)

    print("Generowanie wykresów z wyników benchmarków...")

    lat = load_csv("latency_results.csv")
    if lat:
        plot_latency(lat, fs_filter)

    tp = load_csv("throughput_results.csv")
    if tp:
        plot_throughput(tp, fs_filter)

    pipe = load_csv("pipeline_results.csv")
    if pipe:
        plot_pipeline(pipe, fs_filter)

    mem = load_csv("memory_pressure_results.csv")
    if mem:
        plot_memory(mem)

    print("Gotowe! Wykresy w:", PLOTS_DIR)


if __name__ == "__main__":
    main()
