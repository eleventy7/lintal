#!/usr/bin/env python3
"""Performance benchmark comparing lintal vs checkstyle."""

import subprocess
import statistics
import time
from dataclasses import dataclass
from pathlib import Path

import matplotlib.pyplot as plt

WARMUP_RUNS = 2
TIMED_RUNS = 10

REPOS = [
    ("Agrona", "target/agrona", "config/benchmark/agrona-checkstyle.xml"),
    ("Artio", "target/artio", "config/benchmark/artio-checkstyle.xml"),
    ("Aeron", "target/aeron", "config/benchmark/aeron-checkstyle.xml"),
]

PROJECT_ROOT = Path(__file__).parent.parent
LINTAL_BIN = PROJECT_ROOT / "target/release/lintal"
CHECKSTYLE_JAR = PROJECT_ROOT / "target/checkstyle-13.0.0-all.jar"


@dataclass
class BenchmarkResult:
    name: str
    files: int
    checkstyle_times: list[float]
    lintal_times: list[float]

    @property
    def checkstyle_mean(self) -> float:
        return statistics.mean(self.checkstyle_times)

    @property
    def checkstyle_stdev(self) -> float:
        return statistics.stdev(self.checkstyle_times) if len(self.checkstyle_times) > 1 else 0

    @property
    def lintal_mean(self) -> float:
        return statistics.mean(self.lintal_times)

    @property
    def lintal_stdev(self) -> float:
        return statistics.stdev(self.lintal_times) if len(self.lintal_times) > 1 else 0

    @property
    def speedup(self) -> float:
        return self.checkstyle_mean / self.lintal_mean if self.lintal_mean > 0 else 0


def count_java_files(repo_path: Path) -> int:
    """Count Java files in a repository."""
    return len(list(repo_path.rglob("*.java")))


def run_checkstyle(repo_path: Path, config_path: Path, file_list: Path, suppressions_dir: Path) -> float:
    """Run checkstyle and return elapsed time in seconds."""
    # config_loc points to repo's checkstyle dir for suppressions.xml
    cmd = [
        "java",
        f"-Dconfig_loc={suppressions_dir}",
        "-jar",
        str(CHECKSTYLE_JAR),
        "-c",
        str(config_path),
        f"@{file_list}",
    ]
    start = time.perf_counter()
    subprocess.run(cmd, capture_output=True, check=False)
    return time.perf_counter() - start


def run_lintal(repo_path: Path, config_path: Path, suppressions_dir: Path) -> float:
    """Run lintal and return elapsed time in seconds."""
    cmd = [
        str(LINTAL_BIN),
        "check",
        str(repo_path),
        "--config",
        str(config_path),
        "--config-loc",
        str(suppressions_dir),
    ]
    start = time.perf_counter()
    subprocess.run(cmd, capture_output=True, check=False)
    return time.perf_counter() - start


def create_file_list(repo_path: Path) -> Path:
    """Create a temp file with list of Java files (excluding build/generated)."""
    file_list = PROJECT_ROOT / "target" / f"{repo_path.name}_files.txt"
    # Exclude build directories which contain generated code
    # Use relative path from repo_path to avoid matching lintal's target/ directory
    java_files = sorted(
        f for f in repo_path.rglob("*.java")
        if "/build/" not in str(f.relative_to(repo_path))
    )
    file_list.write_text("\n".join(str(f) for f in java_files))
    return file_list


def benchmark_repo(name: str, repo_rel_path: str, benchmark_config: str) -> BenchmarkResult:
    """Run full benchmark for a single repository."""
    repo_path = PROJECT_ROOT / repo_rel_path
    # Use benchmark config (lintal-supported rules only)
    config_path = PROJECT_ROOT / benchmark_config
    # Suppressions are still in the repo's config dir
    suppressions_dir = repo_path / "config/checkstyle"

    print(f"\n{'='*60}")
    print(f"Benchmarking {name}")
    print(f"{'='*60}")

    java_files = list(repo_path.rglob("*.java"))
    files = len(java_files)
    print(f"Java files: {files}")
    print(f"Config: {benchmark_config}")

    # Create file list for checkstyle
    file_list = create_file_list(repo_path)

    # Warmup runs
    print(f"\nWarmup ({WARMUP_RUNS} runs each)...")
    for i in range(WARMUP_RUNS):
        print(f"  Checkstyle warmup {i+1}...", end=" ", flush=True)
        t = run_checkstyle(repo_path, config_path, file_list, suppressions_dir)
        print(f"{t:.2f}s")

    for i in range(WARMUP_RUNS):
        print(f"  lintal warmup {i+1}...", end=" ", flush=True)
        t = run_lintal(repo_path, config_path, suppressions_dir)
        print(f"{t:.2f}s")

    # Timed runs
    print(f"\nTimed runs ({TIMED_RUNS} each)...")
    checkstyle_times = []
    for i in range(TIMED_RUNS):
        print(f"  Checkstyle run {i+1}...", end=" ", flush=True)
        t = run_checkstyle(repo_path, config_path, file_list, suppressions_dir)
        checkstyle_times.append(t)
        print(f"{t:.2f}s")

    lintal_times = []
    for i in range(TIMED_RUNS):
        print(f"  lintal run {i+1}...", end=" ", flush=True)
        t = run_lintal(repo_path, config_path, suppressions_dir)
        lintal_times.append(t)
        print(f"{t:.2f}s")

    return BenchmarkResult(
        name=name,
        files=files,
        checkstyle_times=checkstyle_times,
        lintal_times=lintal_times,
    )


def print_results_table(results: list[BenchmarkResult]) -> None:
    """Print results in markdown table format."""
    print("\n\n## Results\n")
    print("| Repository | Files | Checkstyle | lintal | Speedup |")
    print("|------------|-------|------------|--------|---------|")
    for r in results:
        print(
            f"| {r.name} | {r.files} | "
            f"{r.checkstyle_mean:.2f}s ± {r.checkstyle_stdev:.2f}s | "
            f"{r.lintal_mean:.2f}s ± {r.lintal_stdev:.2f}s | "
            f"**{r.speedup:.1f}x** |"
        )


def create_chart(results: list[BenchmarkResult], output_path: Path) -> None:
    """Create bar chart with error bars."""
    fig, ax = plt.subplots(figsize=(10, 6))

    repos = [r.name for r in results]
    x = range(len(repos))
    width = 0.35

    checkstyle_means = [r.checkstyle_mean for r in results]
    checkstyle_stdevs = [r.checkstyle_stdev for r in results]
    lintal_means = [r.lintal_mean for r in results]
    lintal_stdevs = [r.lintal_stdev for r in results]

    bars1 = ax.bar(
        [i - width / 2 for i in x],
        checkstyle_means,
        width,
        yerr=checkstyle_stdevs,
        label="Checkstyle",
        color="#e74c3c",
        capsize=5,
    )
    bars2 = ax.bar(
        [i + width / 2 for i in x],
        lintal_means,
        width,
        yerr=lintal_stdevs,
        label="lintal",
        color="#2ecc71",
        capsize=5,
    )

    ax.set_xlabel("Repository")
    ax.set_ylabel("Time (seconds)")
    ax.set_title("lintal vs Checkstyle Performance")
    ax.set_xticks(x)
    ax.set_xticklabels(repos)
    ax.legend()

    # Add speedup annotations
    for i, r in enumerate(results):
        max_height = max(checkstyle_means[i], lintal_means[i]) + max(checkstyle_stdevs[i], lintal_stdevs[i])
        ax.annotate(
            f"{r.speedup:.1f}x faster",
            xy=(i, max_height),
            ha="center",
            va="bottom",
            fontsize=10,
            fontweight="bold",
        )

    plt.tight_layout()
    plt.savefig(output_path, dpi=150)
    print(f"\nChart saved to: {output_path}")


def main() -> None:
    # Check prerequisites
    if not LINTAL_BIN.exists():
        print(f"Error: lintal binary not found at {LINTAL_BIN}")
        print("Run: cargo build --release")
        return

    if not CHECKSTYLE_JAR.exists():
        print(f"Error: checkstyle jar not found at {CHECKSTYLE_JAR}")
        print("Run: mise run download-checkstyle")
        return

    for name, repo_path, _ in REPOS:
        full_path = PROJECT_ROOT / repo_path
        if not full_path.exists():
            print(f"Error: {name} not found at {full_path}")
            print("Run: mise run clone-test-repos")
            return

    # Run benchmarks
    results = []
    for name, repo_path, benchmark_config in REPOS:
        result = benchmark_repo(name, repo_path, benchmark_config)
        results.append(result)

    # Output results
    print_results_table(results)

    # Create chart
    chart_path = PROJECT_ROOT / "target/benchmark_results.png"
    create_chart(results, chart_path)


if __name__ == "__main__":
    main()
