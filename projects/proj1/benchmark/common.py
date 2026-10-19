import os
import sys
import time
import subprocess
import resource
import csv
import re
from pathlib import Path
import matplotlib.pyplot as plt

BENCH_DIR = Path(__file__).resolve().parent
PROJ_DIR = BENCH_DIR.parent
TARGET_DIR = PROJ_DIR / "src" / "target" / "release"
RESULTS_DIR = BENCH_DIR / "results"
PLOTS_DIR = BENCH_DIR / "plots"

def ensure_dirs():
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    PLOTS_DIR.mkdir(parents=True, exist_ok=True)

def create_test_file(path: Path, size_bytes: int):
    if path.exists() and path.stat().st_size == size_bytes:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    chunk = os.urandom(min(16 * 1024 * 1024, size_bytes))
    written = 0
    with open(path, "wb") as f:
        while written < size_bytes:
            w = min(len(chunk), size_bytes - written)
            f.write(chunk[:w])
            written += w
        f.flush()
        os.fsync(f.fileno())

def drop_cache(path: Path):
    try:
        with open(path, "rb") as f:
            if hasattr(os, "posix_fadvise") and hasattr(os, "POSIX_FADV_DONTNEED"):
                os.posix_fadvise(f.fileno(), 0, 0, os.POSIX_FADV_DONTNEED)
    except Exception:
        pass

def warm_cache(path: Path):
    with open(path, "rb") as f:
        while f.read(16 * 1024 * 1024):
            pass

def parse_stderr_metrics(stderr_text: str) -> dict:
    metrics = {}
    for line in stderr_text.splitlines():
        for part in line.split(","):
            if ":" in part:
                k, v = part.split(":", 1)
                k = k.strip()
                v = v.strip()
                try:
                    if "." in v:
                        metrics[k] = float(v)
                    else:
                        metrics[k] = int(v)
                except ValueError:
                    metrics[k] = v
    return metrics

def measure(cmd: list[str], repeats: int = 5, cold: bool = False, file_path: Path | None = None, memory_max: str | None = None) -> dict:
    runs = []
    exec_cmd = cmd
    if memory_max:
        exec_cmd = ["systemd-run", "--user", "--scope", "-p", f"MemoryMax={memory_max}"] + cmd

    for _ in range(repeats):
        if cold and file_path:
            drop_cache(file_path)
        elif not cold and file_path:
            warm_cache(file_path)

        ru_before = resource.getrusage(resource.RUSAGE_CHILDREN)
        t0 = time.perf_counter()
        res = subprocess.run(exec_cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        t1 = time.perf_counter()
        ru_after = resource.getrusage(resource.RUSAGE_CHILDREN)

        if res.returncode != 0:
            raise RuntimeError(f"Błąd uruchomienia {' '.join(exec_cmd)}:\n{res.stderr}")

        wall_s = t1 - t0
        utime_s = ru_after.ru_utime - ru_before.ru_utime
        stime_s = ru_after.ru_stime - ru_before.ru_stime

        metrics = parse_stderr_metrics(res.stderr)
        total_s = metrics.get("czas_całk_s", wall_s)

        run_data = {
            "wall_s": wall_s,
            "total_s": total_s,
            "utime_s": max(0.0, utime_s),
            "stime_s": max(0.0, stime_s),
            **metrics,
        }
        runs.append(run_data)

    return min(runs, key=lambda r: r["total_s"])

def format_bytes(n: int) -> str:
    if n < 1024: return f"{n} B"
    if n < 1024 * 1024: return f"{n // 1024} KB"
    return f"{n // (1024 * 1024)} MB"

def save_csv(data: list[dict], path: Path):
    if not data: return
    with open(path, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=data[0].keys())
        writer.writeheader()
        writer.writerows(data)
