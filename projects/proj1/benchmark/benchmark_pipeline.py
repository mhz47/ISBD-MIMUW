from pathlib import Path
import matplotlib.pyplot as plt
from common import TARGET_DIR, RESULTS_DIR, PLOTS_DIR, measure, format_bytes, save_csv


def run_benchmarks(test_file: Path, k_values: list[int], block_sizes: list[int], repeats: int = 5, memory_max: str | None = None):
    bin_pipe = TARGET_DIR / "pipeline"
    bin_orig = TARGET_DIR / "pipeline_original"
    default_m = 1024 * 1024
    file_size_mb = test_file.stat().st_size / (1024 * 1024)
    results = []

    mem_label = f" (MemoryMax={memory_max})" if memory_max else ""
    print(f"[*] Benchmark Pipeline i K_opt{mem_label}...")

    for cache_state, is_cold in [("warm", False), ("cold", True)]:
        r_count = max(1, repeats - 1) if is_cold else repeats
        print(f"  -> Stan pamięci: {cache_state.upper()} CACHE")

        for k in k_values:
            res_pipe = measure([str(bin_pipe), str(test_file), str(default_m), str(k)], repeats=r_count, cold=is_cold, file_path=test_file, memory_max=memory_max)
            res_orig = measure([str(bin_orig), str(test_file), str(default_m), str(k)], repeats=r_count, cold=is_cold, file_path=test_file, memory_max=memory_max)

            t_pipe = res_pipe["total_s"]
            t_orig = res_orig["total_s"]

            results.append({
                "cache": cache_state, "test": "k_opt", "method": "pipeline", "k": k, "m_bytes": default_m,
                "m_str": format_bytes(default_m), "file_size_mb": file_size_mb, "time_ms": t_pipe * 1000,
                "czas_odczyt_s": res_pipe.get("czas_odczyt_s", 0.0), "czas_oblicz_s": res_pipe.get("czas_oblicz_s", 0.0),
                "utime_s": res_pipe["utime_s"], "stime_s": res_pipe["stime_s"]
            })
            results.append({
                "cache": cache_state, "test": "k_opt", "method": "original", "k": k, "m_bytes": default_m,
                "m_str": format_bytes(default_m), "file_size_mb": file_size_mb, "time_ms": t_orig * 1000,
                "czas_odczyt_s": res_orig.get("czas_odczyt_s", 0.0), "czas_oblicz_s": res_orig.get("czas_oblicz_s", 0.0),
                "utime_s": res_orig["utime_s"], "stime_s": res_orig["stime_s"]
            })
            print(f"     K={k:>5} | Pipeline: {t_pipe*1000:6.2f} ms (odczyt: {res_pipe.get('czas_odczyt_s',0)*1000:5.1f}ms, oblicz: {res_pipe.get('czas_oblicz_s',0)*1000:5.1f}ms) | Sekw: {t_orig*1000:6.2f} ms | Speedup: {t_orig/t_pipe:.2f}x")

    # wpływ M dla różnych K
    print("  -> Badanie wrażliwości na M dla różnych K (Warm Cache)")
    k_for_m = [1, 100, 1000]
    for k in k_for_m:
        for m in block_sizes:
            res_pipe = measure([str(bin_pipe), str(test_file), str(m), str(k)], repeats=repeats, cold=False, file_path=test_file, memory_max=memory_max)
            res_orig = measure([str(bin_orig), str(test_file), str(m), str(k)], repeats=repeats, cold=False, file_path=test_file, memory_max=memory_max)

            results.append({
                "cache": "warm", "test": "m_sens", "method": "pipeline", "k": k, "m_bytes": m,
                "m_str": format_bytes(m), "file_size_mb": file_size_mb, "time_ms": res_pipe["total_s"] * 1000,
                "czas_odczyt_s": res_pipe.get("czas_odczyt_s", 0.0), "czas_oblicz_s": res_pipe.get("czas_oblicz_s", 0.0),
                "utime_s": res_pipe["utime_s"], "stime_s": res_pipe["stime_s"]
            })
            results.append({
                "cache": "warm", "test": "m_sens", "method": "original", "k": k, "m_bytes": m,
                "m_str": format_bytes(m), "file_size_mb": file_size_mb, "time_ms": res_orig["total_s"] * 1000,
                "czas_odczyt_s": res_orig.get("czas_odczyt_s", 0.0), "czas_oblicz_s": res_orig.get("czas_oblicz_s", 0.0),
                "utime_s": res_orig["utime_s"], "stime_s": res_orig["stime_s"]
            })

    return results

def plot_results(results: list[dict]):
    # czas wykonania w funkcji K dla M = 1 MB (przed vs po)
    fig, axes = plt.subplots(1, 2, figsize=(13, 5))
    for ax_idx, (cache_name, title) in enumerate([("warm", "Pamięć RAM (Warm Cache)"), ("cold", "Dysk NVMe (Cold Cache)")]):
        ax = axes[ax_idx]
        data = [r for r in results if r["cache"] == cache_name and r["test"] == "k_opt"]
        k_list = sorted(list(set(r["k"] for r in data)))

        t_pipe = [next(r["time_ms"] for r in data if r["k"] == k and r["method"] == "pipeline") for k in k_list]
        t_orig = [next(r["time_ms"] for r in data if r["k"] == k and r["method"] == "original") for k in k_list]

        ax.plot(k_list, t_orig, marker="s", linewidth=2.2, linestyle="--", color="#e63946", label="Sekwencyjny (przed poprawką)")
        ax.plot(k_list, t_pipe, marker="o", linewidth=2.2, color="#2a9d8f", label="Pipeline producent-konsument (po)")

        ax.set_title(title, fontsize=11, fontweight="bold")
        ax.set_xlabel("Liczba iteracji hashowania (K)", fontsize=10)
        ax.set_ylabel("Całkowity czas wykonania [ms]", fontsize=10)
        ax.legend(fontsize=9)
        ax.grid(True, linestyle="--", alpha=0.6)

    fig.suptitle("Porównanie pipeline (po) z wersją sekwencyjną (przed) w funkcji parametru K (M = 1 MB)", fontsize=12, fontweight="bold")
    plt.tight_layout()
    plt.savefig(PLOTS_DIR / "pipeline_k_opt.png", dpi=200)
    plt.close()

    # dekompozycja czasu pipeline: T_total vs T_read vs T_compute
    fig, axes = plt.subplots(1, 2, figsize=(13, 5))
    for ax_idx, (cache_name, title) in enumerate([("warm", "Pamięć RAM (Warm Cache)"), ("cold", "Dysk NVMe (Cold Cache)")]):
        ax = axes[ax_idx]
        data = [r for r in results if r["cache"] == cache_name and r["test"] == "k_opt" and r["method"] == "pipeline"]
        k_list = sorted(list(set(r["k"] for r in data)))

        t_total = [next(r["time_ms"] for r in data if r["k"] == k) for k in k_list]
        t_read = [next(r["czas_odczyt_s"] * 1000 for r in data if r["k"] == k) for k in k_list]
        t_compute = [next(r["czas_oblicz_s"] * 1000 for r in data if r["k"] == k) for k in k_list]

        ax.plot(k_list, t_total, marker="o", linewidth=2.2, color="#2a9d8f", label="$T_{total}$ (czas całkowity pipeline)")
        ax.plot(k_list, t_read, marker="x", linewidth=1.8, linestyle=":", color="#f4a261", label="$T_{read}$ (czas wątku I/O)")
        ax.plot(k_list, t_compute, marker="^", linewidth=1.8, linestyle="--", color="#9b5de5", label="$T_{compute}$ (czas wątku CPU)")

        ax.set_title(title, fontsize=11, fontweight="bold")
        ax.set_xlabel("Liczba iteracji hashowania (K)", fontsize=10)
        ax.set_ylabel("Czas [ms]", fontsize=10)
        ax.legend(fontsize=9)
        ax.grid(True, linestyle="--", alpha=0.6)

    fig.suptitle("Dekompozycja czasu pipeline: demonstracja reguły T_total ≈ max(T_read, T_compute)", fontsize=12, fontweight="bold")
    plt.tight_layout()
    plt.savefig(PLOTS_DIR / "pipeline_breakdown.png", dpi=200)
    plt.close()

    # czas w funkcji M dla różnych K
    m_sens_data = [r for r in results if r["test"] == "m_sens"]
    if m_sens_data:
        k_list = sorted(list(set(r["k"] for r in m_sens_data)))
        m_list = sorted(list(set(r["m_bytes"] for r in m_sens_data)))
        labels = [format_bytes(m) for m in m_list]
        k_colors = ["#e63946", "#f4a261", "#9b5de5"]

        fig, ax = plt.subplots(figsize=(9, 5))
        for method in ["pipeline", "original"]:
            for k_idx, k in enumerate(k_list):
                style = "-" if method == "pipeline" else "--"
                marker = "o" if method == "pipeline" else "s"
                times = []
                for m in m_list:
                    r = next((x for x in m_sens_data if x["m_bytes"] == m and x["k"] == k and x["method"] == method), None)
                    times.append(r["time_ms"] if r else 0)
                label = f"K={k} ({method})" if k_idx == 0 else f"K={k}"
                ax.plot(range(len(m_list)), times, marker=marker, linestyle=style, linewidth=2,
                        color=k_colors[k_idx % len(k_colors)], label=label)
        ax.set_title("Czas wykonania w funkcji rozmiaru bloku M (Warm Cache)", fontsize=11, fontweight="bold")
        ax.set_xlabel("Rozmiar bloku M", fontsize=10)
        ax.set_ylabel("Całkowity czas wykonania [ms]", fontsize=10)
        ax.set_xticks(range(len(m_list)))
        ax.set_xticklabels(labels, rotation=35)
        ax.legend(fontsize=9)
        ax.grid(True, linestyle="--", alpha=0.6)
        plt.tight_layout()
        plt.savefig(PLOTS_DIR / "pipeline_vs_m.png", dpi=200)
        plt.close()

if __name__ == "__main__":
    from common import ensure_dirs, create_test_file, BENCH_DIR
    ensure_dirs()
    f = BENCH_DIR / "test_data_50mb.bin"
    create_test_file(f, 50 * 1024 * 1024)
    k_vals = [1, 5, 10, 25, 50, 100, 250, 500, 1000, 2000, 5000]
    m_vals = [16384, 65536, 262144, 1048576, 4194304, 16777216]
    res = run_benchmarks(f, k_vals, m_vals, repeats=5)
    save_csv(res, RESULTS_DIR / "pipeline_results.csv")
    plot_results(res)
