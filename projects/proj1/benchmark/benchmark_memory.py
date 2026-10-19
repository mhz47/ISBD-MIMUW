from pathlib import Path
import matplotlib.pyplot as plt
from common import TARGET_DIR, RESULTS_DIR, PLOTS_DIR, measure, format_bytes, save_csv


def run_benchmarks(test_file: Path, block_sizes: list[int], thread_counts: list[int], memory_max: str = "16M", repeats: int = 5):
    """Pomiar zachowania programów przy ograniczeniu pamięci (systemd-run MemoryMax)."""
    bin_lat = TARGET_DIR / "latency"
    bin_lat_orig = TARGET_DIR / "latency_original"
    bin_tp = TARGET_DIR / "throughput"
    bin_tp_orig = TARGET_DIR / "throughput_original"
    results = []

    print(f"[*] Benchmark Presji pamięciowej (MemoryMax={memory_max})...")

    # latency - cold cache, z limitem pamięci
    for cache_state, is_cold in [("cold", True)]:
        r_count = max(1, repeats - 1) if is_cold else repeats
        print(f"  -> Cold Cache, MemoryMax={memory_max}")

        for m in block_sizes:
            # original
            res = measure([str(bin_lat_orig), str(test_file), str(m)], repeats=r_count,
                          cold=is_cold, file_path=test_file, memory_max=memory_max)
            results.append({
                "program": "latency", "version": "original", "m_bytes": m,
                "m_str": format_bytes(m), "threads": 1,
                "time_ms": res["total_s"] * 1000,
                "śr_odczyt_µs": res.get("śr_odczyt_µs", 0.0),
            })
            print(f"     [latency_orig] M={format_bytes(m):>7}, 1T: {res['total_s']*1000:8.1f} ms | śr odczyt: {res.get('śr_odczyt_µs', 0.0):.1f} µs")

            # improved
            for thr in thread_counts:
                res = measure([str(bin_lat), str(test_file), str(m), str(thr)], repeats=r_count,
                              cold=is_cold, file_path=test_file, memory_max=memory_max)
                results.append({
                    "program": "latency", "version": "improved", "m_bytes": m,
                    "m_str": format_bytes(m), "threads": thr,
                    "time_ms": res["total_s"] * 1000,
                    "śr_odczyt_µs": res.get("śr_odczyt_µs", 0.0),
                })
                print(f"     [latency]    M={format_bytes(m):>7}, {thr:>2}T: {res['total_s']*1000:8.1f} ms | śr odczyt: {res.get('śr_odczyt_µs', 0.0):.1f} µs")

    # throughput - cold cache, z limitem pamięci
    print(f"  -> Throughput, Cold Cache, MemoryMax={memory_max}")
    for m in block_sizes:
        res_mmap = measure([str(bin_tp), str(test_file), str(m)], repeats=repeats,
                           cold=True, file_path=test_file, memory_max=memory_max)
        res_read = measure([str(bin_tp_orig), str(test_file), str(m)], repeats=repeats,
                           cold=True, file_path=test_file, memory_max=memory_max)
        results.append({
            "program": "throughput", "version": "mmap", "m_bytes": m,
            "m_str": format_bytes(m), "threads": 1,
            "time_ms": res_mmap["total_s"] * 1000,
            "śr_odczyt_µs": 0.0,
        })
        results.append({
            "program": "throughput", "version": "read", "m_bytes": m,
            "m_str": format_bytes(m), "threads": 1,
            "time_ms": res_read["total_s"] * 1000,
            "śr_odczyt_µs": 0.0,
        })
        print(f"     [throughput] M={format_bytes(m):>7} | mmap: {res_mmap['total_s']*1000:7.1f} ms | read: {res_read['total_s']*1000:7.1f} ms")

    return results


def plot_results(results: list[dict]):
    colors = ["#e63946", "#2a9d8f", "#f4a261", "#9b5de5"]

    # latency - czas całkowity z limitem pamięci
    fig, ax = plt.subplots(figsize=(8, 5))
    lat_data = [r for r in results if r["program"] == "latency"]
    m_list = sorted(list(set(r["m_bytes"] for r in lat_data)))

    for idx, m in enumerate(m_list):
        improved = sorted([r for r in lat_data if r["m_bytes"] == m and r["version"] == "improved"],
                          key=lambda x: x["threads"])
        ax.plot([r["threads"] for r in improved], [r["time_ms"] for r in improved],
                marker="o", linewidth=2, color=colors[idx % len(colors)],
                label=f"M = {format_bytes(m)} (po poprawce)")
        orig = next((r for r in lat_data if r["m_bytes"] == m and r["version"] == "original"), None)
        if orig:
            ax.scatter([1], [orig["time_ms"]], color=colors[idx % len(colors)], marker="X", s=90,
                       edgecolors="black", zorder=5)

    ax.set_title(f"Losowy odczyt przy presji pamięciowej (MemoryMax=16M, Cold Cache)", fontsize=11, fontweight="bold")
    ax.set_xlabel("Liczba wątków (T)", fontsize=10)
    ax.set_ylabel("Całkowity czas wykonania [ms]", fontsize=10)
    ax.set_xticks(sorted(list(set(r["threads"] for r in lat_data if r["version"] == "improved"))))
    ax.grid(True, linestyle="--", alpha=0.6)
    ax.scatter([], [], color="black", marker="X", s=70, label="Przed poprawką (1T)")
    ax.legend(fontsize=9)
    plt.tight_layout()
    plt.savefig(PLOTS_DIR / "memory_pressure_latency.png", dpi=200)
    plt.close()

    # throughput - mmap vs read z limitem pamięci
    fig, ax = plt.subplots(figsize=(8, 5))
    tp_data = [r for r in results if r["program"] == "throughput"]
    m_list = sorted(list(set(r["m_bytes"] for r in tp_data)))
    labels = [format_bytes(m) for m in m_list]

    t_mmap = [next(r["time_ms"] for r in tp_data if r["m_bytes"] == m and r["version"] == "mmap") for m in m_list]
    t_read = [next(r["time_ms"] for r in tp_data if r["m_bytes"] == m and r["version"] == "read") for m in m_list]

    ax.plot(range(len(m_list)), t_read, marker="s", linewidth=2.2, linestyle="--", color="#e63946", label="read()")
    ax.plot(range(len(m_list)), t_mmap, marker="o", linewidth=2.2, color="#2a9d8f", label="mmap()")
    ax.set_title(f"Przepustowość przy presji pamięciowej (MemoryMax=16M, Cold Cache)", fontsize=11, fontweight="bold")
    ax.set_xlabel("Rozmiar bloku M", fontsize=10)
    ax.set_ylabel("Całkowity czas wykonania [ms]", fontsize=10)
    ax.set_xticks(range(len(m_list)))
    ax.set_xticklabels(labels, rotation=35)
    ax.legend(fontsize=9)
    ax.grid(True, linestyle="--", alpha=0.6)
    plt.tight_layout()
    plt.savefig(PLOTS_DIR / "memory_pressure_throughput.png", dpi=200)
    plt.close()


if __name__ == "__main__":
    from common import ensure_dirs, create_test_file, BENCH_DIR
    ensure_dirs()
    f = BENCH_DIR / "test_data_50mb.bin"
    create_test_file(f, 50 * 1024 * 1024)
    res = run_benchmarks(f, [512, 4096, 65536, 1048576], [1, 2, 4, 8], memory_max="16M", repeats=5)
    save_csv(res, RESULTS_DIR / "memory_pressure_results.csv")
    plot_results(res)
