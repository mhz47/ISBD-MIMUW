import sys
from pathlib import Path
import matplotlib.pyplot as plt
from common import TARGET_DIR, RESULTS_DIR, PLOTS_DIR, measure, format_bytes, save_csv

def run_benchmarks(test_file: Path, block_sizes: list[int], thread_counts: list[int], repeats: int = 5, memory_max: str | None = None):
    bin_opt = TARGET_DIR / "latency"
    bin_orig = TARGET_DIR / "latency_original"
    file_size_mb = test_file.stat().st_size / (1024 * 1024)
    results = []

    mem_label = f" (MemoryMax={memory_max})" if memory_max else ""
    print(f"[*] Benchmark Latencji (losowy odczyt){mem_label}...")

    for cache_state, is_cold in [("warm", False), ("cold", True)]:
        r_count = max(1, repeats - 1) if is_cold else repeats
        print(f"  -> Stan pamięci: {cache_state.upper()} CACHE")

        # original 
        for m in block_sizes:
            m_res = measure([str(bin_orig), str(test_file), str(m)], repeats=r_count, cold=is_cold, file_path=test_file, memory_max=memory_max)
            results.append({
                "version": "original", "cache": cache_state, "m_bytes": m, "m_str": format_bytes(m),
                "file_size_mb": file_size_mb, "threads": 1, "time_ms": m_res["total_s"] * 1000,
                "śr_odczyt_µs": m_res.get("śr_odczyt_µs", 0.0),
                "liczba_odczytów": m_res.get("liczba_odczytów", 0),
                "utime_s": m_res["utime_s"], "stime_s": m_res["stime_s"]
            })
            print(f"     [Original] M={format_bytes(m):>7}, 1T: {m_res['total_s']*1000:7.2f} ms | śr odczyt: {m_res.get('śr_odczyt_µs', 0.0):.2f} µs")

        # improved
        for m in block_sizes:
            for thr in thread_counts:
                m_res = measure([str(bin_opt), str(test_file), str(m), str(thr)], repeats=r_count, cold=is_cold, file_path=test_file, memory_max=memory_max)
                results.append({
                    "version": "improved", "cache": cache_state, "m_bytes": m, "m_str": format_bytes(m),
                    "file_size_mb": file_size_mb, "threads": thr, "time_ms": m_res["total_s"] * 1000,
                    "śr_odczyt_µs": m_res.get("śr_odczyt_µs", 0.0),
                    "liczba_odczytów": m_res.get("liczba_odczytów", 0),
                    "utime_s": m_res["utime_s"], "stime_s": m_res["stime_s"]
                })
                print(f"     [Improved] M={format_bytes(m):>7}, {thr:>2}T: {m_res['total_s']*1000:7.2f} ms | śr odczyt: {m_res.get('śr_odczyt_µs', 0.0):.2f} µs")

    return results

def plot_results(results: list[dict]):
    colors = ["#e63946", "#2a9d8f", "#f4a261", "#9b5de5"]
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

    all_threads = sorted(list(set(int(r["threads"]) for r in results if r["version"] == "improved")))
    t_min = min(all_threads)
    t_max = max(all_threads)
    file_sizes = sorted(list(set(float(r.get("file_size_mb", 50)) for r in results)))

    # czas całkowity
    m_list = sorted(list(set(int(r["m_bytes"]) for r in results)))
    fig, axes = plt.subplots(2, len(m_list), figsize=(4*len(m_list), 8))
    if len(m_list) == 1:
        axes = axes.reshape(2, 1)

    for m_idx, m in enumerate(m_list):
        m_label = format_bytes(m)
        color = get_color(m)

        for row_idx, (cache_name, cache_title) in enumerate([("warm", "Warm"), ("cold", "Cold")]):
            ax = axes[row_idx, m_idx]
            data = [r for r in results if r["cache"] == cache_name]
            if not data:
                continue

            opt = sorted([r for r in data if int(r["m_bytes"]) == m and r["version"] == "improved"],
                         key=lambda x: int(x["threads"]))
            if opt:
                ax.plot([int(r["threads"]) for r in opt], [float(r["time_ms"]) for r in opt],
                        marker="o", linewidth=2.5, color=color, markersize=7, label="improved")

            orig = next((r for r in data if int(r["m_bytes"]) == m and r["version"] == "original"), None)
            if orig:
                ax.hlines(float(orig["time_ms"]), t_min, t_max,
                          colors=color, linestyles="--", linewidth=2, alpha=0.7, label="original")

            if row_idx == 0:
                ax.set_title(f"M={m_label}", fontsize=11, fontweight="bold")
            if m_idx == 0:
                ax.set_ylabel(f"{cache_title}\n[ms]", fontsize=10)

            ax.set_xlabel("Wątki (T)", fontsize=10)
            ax.set_xticks(all_threads)
            ax.legend(fontsize=9, loc="upper right")
            ax.grid(True, linestyle="--", alpha=0.4)

    fs_text = file_sizes[0] if len(file_sizes) == 1 else "różny"
    fig.suptitle(f"Czas losowego odczytu w funkcji wątków (plik {fs_text} MB)", fontsize=13, fontweight="bold")
    plt.tight_layout(rect=[0, 0, 0.97, 0.95])
    plt.savefig(PLOTS_DIR / "latency_time_vs_threads.png", dpi=200, bbox_inches="tight")
    plt.close()

    # sredni czas odczytu
    fig, axes = plt.subplots(2, len(m_list), figsize=(4*len(m_list), 8))
    if len(m_list) == 1:
        axes = axes.reshape(2, 1)

    for m_idx, m in enumerate(m_list):
        m_label = format_bytes(m)
        color = get_color(m)

        for row_idx, (cache_name, cache_title) in enumerate([("warm", "Warm"), ("cold", "Cold")]):
            ax = axes[row_idx, m_idx]
            data = [r for r in results if r["cache"] == cache_name]
            if not data:
                continue

            opt = sorted([r for r in data if int(r["m_bytes"]) == m and r["version"] == "improved"],
                         key=lambda x: int(x["threads"]))
            if opt:
                ax.plot([int(r["threads"]) for r in opt], [float(r["śr_odczyt_µs"]) for r in opt],
                        marker="o", linewidth=2.5, color=color, markersize=7, label="improved")

            orig = next((r for r in data if int(r["m_bytes"]) == m and r["version"] == "original"), None)
            if orig:
                ax.hlines(float(orig["śr_odczyt_µs"]), t_min, t_max,
                          colors=color, linestyles="--", linewidth=2, alpha=0.7, label="original")

            if row_idx == 0:
                ax.set_title(f"M={m_label}", fontsize=11, fontweight="bold")
            if m_idx == 0:
                ax.set_ylabel(f"{cache_title}\n[µs]", fontsize=10)

            ax.set_xlabel("Wątki (T)", fontsize=10)
            ax.set_xticks(all_threads)
            ax.legend(fontsize=9, loc="upper right")
            ax.grid(True, linestyle="--", alpha=0.4)

    fig.suptitle(f"Średni czas pread() w funkcji wątków (plik {fs_text} MB)", fontsize=13, fontweight="bold")
    plt.tight_layout(rect=[0, 0, 0.97, 0.95])
    plt.savefig(PLOTS_DIR / "latency_read_vs_threads.png", dpi=200, bbox_inches="tight")
    plt.close()

    # czas całkowity w funkcji M
    if len(file_sizes) > 1:
        fig, axes = plt.subplots(1, 2, figsize=(13, 5))
        for ax_idx, (cache_name, title) in enumerate([("warm", "Pamięć RAM (Warm Cache)"), ("cold", "Dysk NVMe (Cold Cache)")]):
            ax = axes[ax_idx]
            m_list = sorted(list(set(int(r["m_bytes"]) for r in results)))
            labels = [format_bytes(m) for m in m_list]
            fs_colors = ["#e63946", "#f4a261", "#2a9d8f"]
            for fs_idx, fs in enumerate(file_sizes):
                data = [r for r in results if r["cache"] == cache_name and r.get("file_size_mb", 50) == fs
                        and r["version"] == "improved" and int(r["threads"]) == 1]
                if not data:
                    continue
                times = [next((float(r["time_ms"]) for r in data if int(r["m_bytes"]) == m), None) for m in m_list]
                times = [t if t is not None else 0 for t in times]
                ax.plot(range(len(m_list)), times, marker="o", linewidth=2,
                        color=fs_colors[fs_idx % len(fs_colors)],
                        label=f"Plik {int(float(fs))} MB")
            ax.set_title(title, fontsize=11, fontweight="bold")
            ax.set_xlabel("Rozmiar bloku M", fontsize=10)
            ax.set_ylabel("Całkowity czas wykonania [ms]", fontsize=10)
            ax.set_xticks(range(len(m_list)))
            ax.set_xticklabels(labels, rotation=35)
            ax.legend(fontsize=9)
            ax.grid(True, linestyle="--", alpha=0.6)
        fig.suptitle("Wpływ rozmiaru pliku na czas losowego odczytu (1 wątek)", fontsize=12, fontweight="bold")
        plt.tight_layout()
        plt.savefig(PLOTS_DIR / "latency_vs_filesize.png", dpi=200)
        plt.close()


if __name__ == "__main__":
    from common import ensure_dirs, create_test_file, BENCH_DIR
    ensure_dirs()
    f = BENCH_DIR / "test_data_50mb.bin"
    create_test_file(f, 50 * 1024 * 1024)
    res = run_benchmarks(f, [512, 4096, 65536, 1048576], [1, 2, 4, 8, 16, 32], repeats=5)
    save_csv(res, RESULTS_DIR / "latency_results.csv")
    plot_results(res)
