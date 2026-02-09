#!/usr/bin/env python3
"""
Binary size tracking and comparison for Rust projects.

Supports both PR validation (comparing sizes) and release tracking (recording to metrics).
Designed to be template-friendly and portable across Rust projects.

Usage:
    # Compare two sizes directly (PR validation in CI)
    python binary-size-diff.py compare 5500000 5400000 --github-output "$GITHUB_OUTPUT"

    # Compare binary against last recorded size in metrics file
    python binary-size-diff.py compare --binary target/release/app --metrics metrics/binary-size.txt

    # Record binary size to metrics file (release tracking)
    python binary-size-diff.py record --binary target/release/app --metrics metrics/binary-size.txt

    # Get size of a single binary
    python binary-size-diff.py size target/release/app
"""

from __future__ import annotations

import argparse
import sys
from datetime import datetime
from pathlib import Path


def human_size(size_bytes: int) -> str:
    """Convert bytes to human-readable format (e.g., 5.2MB)."""
    if size_bytes == 0:
        return "0B"

    sign = "-" if size_bytes < 0 else ""
    size_bytes = abs(size_bytes)

    for unit in ["B", "KB", "MB", "GB"]:
        if size_bytes < 1024:
            if unit == "B":
                return f"{sign}{size_bytes}{unit}"
            return f"{sign}{size_bytes:.1f}{unit}"
        size_bytes /= 1024

    return f"{sign}{size_bytes:.1f}TB"


def get_file_size(path: str | Path) -> int:
    """Get file size in bytes."""
    return Path(path).stat().st_size


def read_last_size_from_metrics(metrics_file: str | Path) -> int | None:
    """Read the last recorded size from a metrics file."""
    path = Path(metrics_file)
    if not path.exists():
        return None

    lines = path.read_text().strip().split("\n")
    for line in reversed(lines):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        # Format: DATE LABEL SIZE HUMAN
        parts = line.split()
        if len(parts) >= 3:
            try:
                return int(parts[2])
            except ValueError:
                continue
    return None


def record_size_to_metrics(
    metrics_file: str | Path,
    size: int,
    label: str = "binary",
) -> None:
    """Append size record to metrics file."""
    path = Path(metrics_file)
    path.parent.mkdir(parents=True, exist_ok=True)

    date = datetime.now().strftime("%Y-%m-%d")
    human = human_size(size)
    line = f"{date} {label} {size} {human}\n"

    with open(path, "a") as f:
        f.write(line)


def calculate_diff(
    current_size: int,
    baseline_size: int,
    threshold_percent: float = 5.0,
    threshold_bytes: int = 51200,
) -> dict:
    """
    Calculate size difference and determine if it's significant.

    Args:
        current_size: Size of current binary in bytes
        baseline_size: Size of baseline binary in bytes
        threshold_percent: Percentage change to consider significant (default 5%)
        threshold_bytes: Absolute byte change to consider significant (default 50KB)

    Returns:
        Dictionary with diff information
    """
    diff = current_size - baseline_size

    current_human = human_size(current_size)
    baseline_human = human_size(baseline_size)

    # Format diff with sign
    diff_human = human_size(diff)
    if diff > 0:
        diff_human = f"+{diff_human}"

    # Calculate percentage
    if baseline_size > 0:
        percent = (diff / baseline_size) * 100
        percent_str = f"+{percent:.1f}%" if diff > 0 else f"{percent:.1f}%"
    else:
        percent = 0.0
        percent_str = "N/A"

    # Determine significance (exceeds either threshold)
    abs_diff = abs(diff)
    abs_percent = abs(percent)
    significant = abs_diff > threshold_bytes or abs_percent > threshold_percent

    # Any change at all (for release tracking)
    changed = diff != 0

    return {
        "current_size": current_human,
        "current_size_bytes": current_size,
        "baseline_size": baseline_human,
        "baseline_size_bytes": baseline_size,
        "diff": diff_human,
        "diff_bytes": diff,
        "percent": percent_str,
        "percent_value": percent,
        "significant": significant,
        "changed": changed,
        # Aliases for PR workflow compatibility
        "pr_size": current_human,
        "main_size": baseline_human,
        "size": current_human,
        "prev_size": baseline_human,
    }


def write_github_output(results: dict, output_file: str | None) -> None:
    """Write results to GITHUB_OUTPUT file or stdout."""
    lines = [
        f"size={results['current_size']}",
        f"size_bytes={results['current_size_bytes']}",
        f"prev_size={results['baseline_size']}",
        f"prev_size_bytes={results['baseline_size_bytes']}",
        f"pr_size={results['current_size']}",
        f"main_size={results['baseline_size']}",
        f"diff={results['diff']}",
        f"diff_bytes={results['diff_bytes']}",
        f"percent={results['percent']}",
        f"significant={'true' if results['significant'] else 'false'}",
        f"changed={'true' if results['changed'] else 'false'}",
    ]

    if output_file:
        with open(output_file, "a") as f:
            f.write("\n".join(lines) + "\n")
    else:
        for line in lines:
            print(line)


def print_table(
    results: dict, current_label: str = "Current", baseline_label: str = "Baseline"
) -> None:
    """Print a human-readable table (for local use)."""
    print()
    print("┌─────────────┬──────────────────────┐")
    print("│ Metric      │ Value                │")
    print("├─────────────┼──────────────────────┤")
    print(f"│ {current_label:<11} │ {results['current_size']:>20} │")
    print(f"│ {baseline_label:<11} │ {results['baseline_size']:>20} │")
    print(f"│ Change      │ {results['diff'] + ' (' + results['percent'] + ')':>20} │")
    print(f"│ Significant │ {str(results['significant']):>20} │")
    print("└─────────────┴──────────────────────┘")
    if results["significant"]:
        print("\n⚠️  Size change exceeds threshold!")
    print()


def cmd_size(args: argparse.Namespace) -> int:
    """Get size of a single binary."""
    size = get_file_size(args.binary)
    if args.human:
        print(human_size(size))
    else:
        print(size)
    return 0


def cmd_compare(args: argparse.Namespace) -> int:
    """Compare two binary sizes."""
    current_size: int | None = None
    baseline_size: int | None = None

    # Determine current size
    if args.current_size is not None:
        current_size = args.current_size
    elif args.binary:
        current_size = get_file_size(args.binary)
    else:
        print("Error: provide current size or --binary", file=sys.stderr)
        return 1

    # Determine baseline size
    if args.baseline_size is not None:
        baseline_size = args.baseline_size
    elif args.baseline:
        baseline_size = get_file_size(args.baseline)
    elif args.metrics:
        baseline_size = read_last_size_from_metrics(args.metrics)
        if baseline_size is None:
            print(f"Warning: no previous size in {args.metrics}", file=sys.stderr)
            baseline_size = 0
    else:
        print("Error: provide baseline size, --baseline, or --metrics", file=sys.stderr)
        return 1

    # Calculate difference
    results = calculate_diff(
        current_size,
        baseline_size,
        threshold_percent=args.threshold_percent,
        threshold_bytes=args.threshold_bytes,
    )

    # Output results
    if args.json:
        import json

        print(json.dumps(results, indent=2))
    elif args.github_output:
        write_github_output(results, args.github_output)
    else:
        current_label = "This PR" if not args.binary else "Current"
        baseline_label = "main" if not args.baseline else "Baseline"
        print_table(results, current_label, baseline_label)

    return 0


def cmd_record(args: argparse.Namespace) -> int:
    """Record binary size to metrics file."""
    size = get_file_size(args.binary)
    human = human_size(size)

    # Get previous size for comparison
    prev_size = read_last_size_from_metrics(args.metrics)

    # Record new size
    record_size_to_metrics(args.metrics, size, args.label)
    print(f"Recorded: {args.label} {size} ({human})")

    # Output for GitHub Actions if requested
    if args.github_output:
        if prev_size is not None and prev_size > 0:
            results = calculate_diff(size, prev_size)
            write_github_output(results, args.github_output)
        else:
            # First record, no comparison
            lines = [
                f"size={human}",
                f"size_bytes={size}",
                "changed=false",
            ]
            with open(args.github_output, "a") as f:
                f.write("\n".join(lines) + "\n")

    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Binary size tracking and comparison for Rust projects",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # size command
    size_parser = subparsers.add_parser("size", help="Get size of a binary")
    size_parser.add_argument("binary", help="Path to binary file")
    size_parser.add_argument(
        "--human", "-H", action="store_true", help="Human-readable output"
    )

    # compare command
    compare_parser = subparsers.add_parser("compare", help="Compare two binary sizes")
    compare_parser.add_argument(
        "current_size", type=int, nargs="?", help="Current size in bytes"
    )
    compare_parser.add_argument(
        "baseline_size", type=int, nargs="?", help="Baseline size in bytes"
    )
    compare_parser.add_argument(
        "--binary", help="Path to current binary (instead of size arg)"
    )
    compare_parser.add_argument(
        "--baseline", help="Path to baseline binary (instead of size arg)"
    )
    compare_parser.add_argument("--metrics", help="Read baseline from metrics file")
    compare_parser.add_argument(
        "--threshold-percent",
        type=float,
        default=5.0,
        help="Percentage threshold for significant change (default: 5.0)",
    )
    compare_parser.add_argument(
        "--threshold-bytes",
        type=int,
        default=51200,
        help="Byte threshold for significant change (default: 50KB)",
    )
    compare_parser.add_argument(
        "--github-output", metavar="FILE", help="Write to GITHUB_OUTPUT file"
    )
    compare_parser.add_argument("--json", action="store_true", help="JSON output")

    # record command
    record_parser = subparsers.add_parser(
        "record", help="Record binary size to metrics file"
    )
    record_parser.add_argument("--binary", required=True, help="Path to binary file")
    record_parser.add_argument("--metrics", required=True, help="Path to metrics file")
    record_parser.add_argument(
        "--label", default="binary", help="Label for this binary (default: binary)"
    )
    record_parser.add_argument(
        "--github-output", metavar="FILE", help="Write comparison to GITHUB_OUTPUT file"
    )

    args = parser.parse_args()

    if args.command == "size":
        return cmd_size(args)
    elif args.command == "compare":
        return cmd_compare(args)
    elif args.command == "record":
        return cmd_record(args)

    return 1


if __name__ == "__main__":
    sys.exit(main())
