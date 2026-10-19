import sys
import time
from common import ensure_dirs, create_test_file, save_csv, BENCH_DIR, RESULTS_DIR
import benchmark_latency
import benchmark_throughput
import benchmark_pipeline
import benchmark_memory


def main():
    ensure_dirs()
    repeats = 5
    file_sizes_mb = [10, 50, 200]
    file_sizes_limited_mb = [500]
    thread_counts = [1, 2, 4, 8, 16, 32]

    t0 = time.perf_counter()
    all_sizes = file_sizes_mb + file_sizes_limited_mb
    print("=" * 60)
    print(f"  BENCHMARKI PROJEKT 1 (Rozmiary: {all_sizes} MB, Powtórzenia: {repeats})")
    print("=" * 60)

    all_latency = []
    all_throughput = []
    all_pipeline = []

    for size_mb in file_sizes_mb:
        test_file = BENCH_DIR / f"test_data_{size_mb}mb.bin"
        create_test_file(test_file, size_mb * 1024 * 1024)
        file_size_bytes = size_mb * 1024 * 1024

        print(f"\n{'='*60}")
        print(f"  POMIARY DLA PLIKU: {size_mb} MB")
        print(f"{'='*60}")

        latency_blocks = [m for m in [512, 4096, 65536, 1048576, 16777216] if m <= file_size_bytes]
        throughput_blocks = [m for m in [512, 1024, 4096, 16384, 65536, 262144, 1048576, 4194304, 16777216] if m <= file_size_bytes]
        pipeline_blocks = [m for m in [16384, 65536, 262144, 1048576, 4194304, 16777216] if m <= file_size_bytes]

        # latencja
        lat_res = benchmark_latency.run_benchmarks(
            test_file,
            latency_blocks,
            thread_counts,
            repeats,
        )
        all_latency.extend(lat_res)

        # przepustowość
        tp_res = benchmark_throughput.run_benchmarks(
            test_file,
            throughput_blocks,
            repeats,
        )
        all_throughput.extend(tp_res)

        # pipeline
        pipe_res = benchmark_pipeline.run_benchmarks(
            test_file,
            [1, 5, 10, 25, 50, 100, 250, 500, 1000, 2000, 5000],
            pipeline_blocks,
            repeats,
        )
        all_pipeline.extend(pipe_res)

    # ograniczenie cache (MemoryMax=256M)
    memory_limit = "256M"
    for size_mb in file_sizes_limited_mb:
        test_file = BENCH_DIR / f"test_data_{size_mb}mb.bin"
        create_test_file(test_file, size_mb * 1024 * 1024)
        file_size_bytes = size_mb * 1024 * 1024

        print(f"\n{'='*60}")
        print(f"  POMIARY DLA PLIKU: {size_mb} MB (MemoryMax={memory_limit} — plik nie mieści się w cache)")
        print(f"{'='*60}")

        latency_blocks = [m for m in [512, 4096, 65536, 1048576, 16777216] if m <= file_size_bytes]
        throughput_blocks = [m for m in [512, 1024, 4096, 16384, 65536, 262144, 1048576, 4194304, 16777216] if m <= file_size_bytes]
        pipeline_blocks = [m for m in [16384, 65536, 262144, 1048576, 4194304, 16777216] if m <= file_size_bytes]

        # latencja
        lat_res = benchmark_latency.run_benchmarks(
            test_file,
            latency_blocks,
            thread_counts,
            repeats,
            memory_max=memory_limit,
        )
        all_latency.extend(lat_res)

        # przepustowość
        tp_res = benchmark_throughput.run_benchmarks(
            test_file,
            throughput_blocks,
            repeats,
            memory_max=memory_limit,
        )
        all_throughput.extend(tp_res)

        # pipeline
        pipe_res = benchmark_pipeline.run_benchmarks(
            test_file,
            [1, 5, 10, 25, 50, 100, 250, 500, 1000, 2000, 5000],
            pipeline_blocks,
            repeats,
            memory_max=memory_limit,
        )
        all_pipeline.extend(pipe_res)

    save_csv(all_latency, RESULTS_DIR / "latency_results.csv")
    benchmark_latency.plot_results(all_latency)

    save_csv(all_throughput, RESULTS_DIR / "throughput_results.csv")
    benchmark_throughput.plot_results(all_throughput)

    save_csv(all_pipeline, RESULTS_DIR / "pipeline_results.csv")
    benchmark_pipeline.plot_results(all_pipeline)

    # memory pressure
    test_file_50 = BENCH_DIR / "test_data_50mb.bin"
    create_test_file(test_file_50, 50 * 1024 * 1024)
    mem_res = benchmark_memory.run_benchmarks(
        test_file_50,
        [512, 4096, 65536, 1048576],
        thread_counts,
        memory_max="16M",
        repeats=repeats,
    )
    save_csv(mem_res, RESULTS_DIR / "memory_pressure_results.csv")
    benchmark_memory.plot_results(mem_res)

    print("=" * 60)
    print(f"[+] Zakończono wszystkie pomiary w {time.perf_counter() - t0:.2f} s")
    print("=" * 60)


if __name__ == "__main__":
    main()
