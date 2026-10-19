from pathlib import Path
import matplotlib.pyplot as plt
from common import TARGET_DIR, RESULTS_DIR, PLOTS_DIR, measure, format_bytes, save_csv


def run_benchmarks(test_file: Path, block_sizes: list[int], repeats: int = 5, memory_max: str | None = None):
    bin_mmap = TARGET_DIR / "throughput"
    bin_read = TARGET_DIR / "throughput_original"
    file_size_mb = test_file.stat().st_size / (1024 * 1024)
    results = []

    mem_label = f" (MemoryMax={memory_max})" if memory_max else ""
    print(f"[*] Benchmark Przepustowości (mmap vs read){mem_label}...")

    for cache_state, is_cold in [("warm", False), ("cold", True)]:
        r_count = max(1, repeats - 1) if is_cold else repeats
        print(f"  -> Stan pamięci: {cache_state.upper()} CACHE")

        for m in block_sizes:
            res_mmap = measure([str(bin_mmap), str(test_file), str(m)], repeats=r_count, cold=is_cold, file_path=test_file, memory_max=memory_max)
            res_read = measure([str(bin_read), str(test_file), str(m)], repeats=r_count, cold=is_cold, file_path=test_file, memory_max=memory_max)

            t_mmap = res_mmap["total_s"]
            t_read = res_read["total_s"]

            results.append({
                "cache": cache_state, "method": "mmap", "m_bytes": m, "m_str": format_bytes(m),
                "file_size_mb": file_size_mb, "time_ms": t_mmap * 1000, "speed_mbs": file_size_mb / t_mmap,
                "stime_s": res_mmap["stime_s"], "utime_s": res_mmap["utime_s"],
                "czas_odczyt_s": res_mmap.get("czas_odczyt_s", 0.0)
            })
            results.append({
                "cache": cache_state, "method": "read", "m_bytes": m, "m_str": format_bytes(m),
                "file_size_mb": file_size_mb, "time_ms": t_read * 1000, "speed_mbs": file_size_mb / t_read,
                "stime_s": res_read["stime_s"], "utime_s": res_read["utime_s"],
                "czas_odczyt_s": res_read.get("czas_odczyt_s", 0.0)
            })
            print(f"     M={format_bytes(m):>7} | mmap: {t_mmap*1000:6.2f} ms | read: {t_read*1000:6.2f} ms | speedup: {t_read/t_mmap:.2f}x | sys: {res_read['stime_s']*1000:5.1f}ms vs {res_mmap['stime_s']*1000:5.1f}ms")

    return results


def plot_results(results: list[dict]):
    # czas całkowity w funkcji M
    fig, axes = plt.subplots(1, 2, figsize=(13, 5))
    for ax_idx, (cache_name, title) in enumerate([("warm", "Pamięć RAM (Warm Cache)"), ("cold", "Dysk NVMe (Cold Cache)")]):
        ax = axes[ax_idx]
        data = [r for r in results if r["cache"] == cache_name]
        m_list = sorted(list(set(r["m_bytes"] for r in data)))
        labels = [format_bytes(m) for m in m_list]

        t_mmap = [next(r["time_ms"] for r in data if r["m_bytes"] == m and r["method"] == "mmap") for m in m_list]
        t_read = [next(r["time_ms"] for r in data if r["m_bytes"] == m and r["method"] == "read") for m in m_list]

        ax.plot(range(len(m_list)), t_read, marker="s", linewidth=2.2, linestyle="--", color="#e63946", label="read() [przed poprawką]")
        ax.plot(range(len(m_list)), t_mmap, marker="o", linewidth=2.2, color="#2a9d8f", label="mmap() [po poprawce]")

        ax.set_title(title, fontsize=11, fontweight="bold")
        ax.set_xlabel("Rozmiar bloku M", fontsize=10)
        ax.set_ylabel("Całkowity czas wykonania [ms]", fontsize=10)
        ax.set_xticks(range(len(m_list)))
        ax.set_xticklabels(labels, rotation=35)
        ax.legend(fontsize=9)
        ax.grid(True, linestyle="--", alpha=0.6)

    fig.suptitle("Porównanie czasu wykonania: read() (przed) vs mmap() (po) w funkcji wielkości bloku M", fontsize=12, fontweight="bold")
    plt.tight_layout()
    plt.savefig(PLOTS_DIR / "throughput_mmap_vs_read.png", dpi=200)
    plt.close()

    # czas spędzony w jądrze (narzut wywołań systemowych) w funkcji M
    fig, axes = plt.subplots(1, 2, figsize=(13, 5))
    for ax_idx, (cache_name, title) in enumerate([("warm", "Pamięć RAM (Warm Cache)"), ("cold", "Dysk NVMe (Cold Cache)")]):
        ax = axes[ax_idx]
        data = [r for r in results if r["cache"] == cache_name]
        m_list = sorted(list(set(r["m_bytes"] for r in data)))
        labels = [format_bytes(m) for m in m_list]

        s_mmap = [next(r["stime_s"] * 1000 for r in data if r["m_bytes"] == m and r["method"] == "mmap") for m in m_list]
        s_read = [next(r["stime_s"] * 1000 for r in data if r["m_bytes"] == m and r["method"] == "read") for m in m_list]

        ax.plot(range(len(m_list)), s_read, marker="s", linewidth=2.2, linestyle="--", color="#e63946", label="read() [czas jądra (sys)]")
        ax.plot(range(len(m_list)), s_mmap, marker="o", linewidth=2.2, color="#2a9d8f", label="mmap() [czas jądra (sys)]")

        ax.set_title(title, fontsize=11, fontweight="bold")
        ax.set_xlabel("Rozmiar bloku M", fontsize=10)
        ax.set_ylabel("Czas w przestrzeni jądra (sys) [ms]", fontsize=10)
        ax.set_xticks(range(len(m_list)))
        ax.set_xticklabels(labels, rotation=35)
        ax.legend(fontsize=9)
        ax.grid(True, linestyle="--", alpha=0.6)

    fig.suptitle("Narzut wywołań systemowych: czas jądra CPU dla read() vs mmap()", fontsize=12, fontweight="bold")
    plt.tight_layout()
    plt.savefig(PLOTS_DIR / "throughput_sys_time.png", dpi=200)
    plt.close()

    # wpływ rozmiaru pliku na czas wykonania (mmap vs read)
    file_sizes = sorted(list(set(r.get("file_size_mb", 50) for r in results)))
    if len(file_sizes) > 1:
        fig, axes = plt.subplots(1, 2, figsize=(13, 5))
        for ax_idx, (cache_name, title) in enumerate([("warm", "Pamięć RAM (Warm Cache)"), ("cold", "Dysk NVMe (Cold Cache)")]):
            ax = axes[ax_idx]
            m_list = sorted(list(set(r["m_bytes"] for r in results)))
            labels = [format_bytes(m) for m in m_list]
            fs_colors = ["#e63946", "#f4a261", "#2a9d8f"]
            for method in ["read", "mmap"]:
                for fs_idx, fs in enumerate(file_sizes):
                    data = [r for r in results if r["cache"] == cache_name and r["method"] == method
                            and r.get("file_size_mb", 50) == fs]
                    if not data:
                        continue
                    times = [next((r["time_ms"] for r in data if r["m_bytes"] == m), None) for m in m_list]
                    times = [t if t is not None else 0 for t in times]
                    style = "--" if method == "read" else "-"
                    marker = "s" if method == "read" else "o"
                    label = f"{method}() {int(fs)} MB" if fs_idx == 0 else f"{int(fs)} MB"
                    ax.plot(range(len(m_list)), times, marker=marker, linestyle=style, linewidth=2,
                            color=fs_colors[fs_idx % len(fs_colors)], label=label)
            ax.set_title(title, fontsize=11, fontweight="bold")
            ax.set_xlabel("Rozmiar bloku M", fontsize=10)
            ax.set_ylabel("Całkowity czas wykonania [ms]", fontsize=10)
            ax.set_xticks(range(len(m_list)))
            ax.set_xticklabels(labels, rotation=35)
            ax.legend(fontsize=8)
            ax.grid(True, linestyle="--", alpha=0.6)
        fig.suptitle("Wpływ rozmiaru pliku na czas sekwencyjnego odczytu (mmap vs read)", fontsize=12, fontweight="bold")
        plt.tight_layout()
        plt.savefig(PLOTS_DIR / "throughput_vs_filesize.png", dpi=200)
        plt.close()


if __name__ == "__main__":
    from common import ensure_dirs, create_test_file, BENCH_DIR
    ensure_dirs()
    f = BENCH_DIR / "test_data_50mb.bin"
    create_test_file(f, 50 * 1024 * 1024)
    res = run_benchmarks(f, [512, 1024, 4096, 16384, 65536, 262144, 1048576, 4194304, 16777216], repeats=5)
    save_csv(res, RESULTS_DIR / "throughput_results.csv")
    plot_results(res)
