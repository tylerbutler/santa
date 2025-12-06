#!/usr/bin/env python3
"""Collect packages from all configured sources.

Usage:
    python collect_all.py                           # Run all collectors
    python collect_all.py --sources homebrew toolleeo
    python collect_all.py --homebrew-limit 200
    python collect_all.py --list                    # List available collectors
"""

import argparse
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

# Add scripts directory to path for imports
sys.path.insert(0, str(Path(__file__).parent))

from collectors.homebrew import HomebrewCollector
from collectors.toolleeo import ToolleeoCollector
from collectors.modern_unix import ModernUnixCollector
from collectors.scoop import ScoopCollector
from collectors.aur import AURCollector
from collectors.arch import ArchCollector
from collectors.awesome_cli_apps import AwesomeCliAppsCollector


COLLECTORS = {
    "homebrew": HomebrewCollector,
    "toolleeo": ToolleeoCollector,
    "modern_unix": ModernUnixCollector,
    "scoop": ScoopCollector,
    "aur": AURCollector,
    "arch": ArchCollector,
    "awesome_cli_apps": AwesomeCliAppsCollector,
}


def main():
    parser = argparse.ArgumentParser(
        description="Collect packages from various sources",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    %(prog)s                              # Run all collectors
    %(prog)s --sources homebrew toolleeo  # Run specific collectors
    %(prog)s --homebrew-limit 200         # Limit Homebrew to 200 packages
    %(prog)s --skip scoop                  # Skip slow collectors
        """,
    )
    parser.add_argument(
        "--sources",
        nargs="+",
        choices=list(COLLECTORS.keys()) + ["all"],
        default=["all"],
        help="Sources to collect from (default: all)",
    )
    parser.add_argument(
        "--skip",
        nargs="+",
        choices=list(COLLECTORS.keys()),
        default=[],
        help="Sources to skip",
    )
    parser.add_argument(
        "--homebrew-limit",
        type=int,
        default=500,
        help="Limit for Homebrew packages (default: 500)",
    )
    parser.add_argument(
        "--arch-limit",
        type=int,
        default=200,
        help="Limit for Arch packages (default: 200)",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=None,
        help="General limit for all collectors (default: none)",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="List available collectors and exit",
    )

    args = parser.parse_args()

    if args.list:
        print("Available collectors:")
        for name in COLLECTORS:
            print(f"  - {name}")
        return 0

    # Determine which collectors to run
    if "all" in args.sources:
        sources_to_run = list(COLLECTORS.keys())
    else:
        sources_to_run = args.sources

    # Remove skipped sources
    sources_to_run = [s for s in sources_to_run if s not in args.skip]

    if not sources_to_run:
        print("No sources selected to run")
        return 1

    print(f"Running collectors in parallel: {', '.join(sources_to_run)}\n")

    def run_collector(source_name: str) -> tuple[str, dict]:
        """Run a single collector and return results."""
        collector_class = COLLECTORS[source_name]
        collector = collector_class()

        # Determine limit for this collector
        if source_name == "homebrew":
            limit = args.homebrew_limit
        elif source_name == "arch":
            limit = args.arch_limit
        else:
            limit = args.limit

        try:
            packages, output_path = collector.run(limit=limit)
            return source_name, {
                "count": len(packages),
                "output": str(output_path),
                "errors": collector.errors,
            }
        except Exception as e:
            return source_name, {"count": 0, "output": None, "errors": [str(e)]}

    results = {}
    total_packages = 0

    with ThreadPoolExecutor(max_workers=len(sources_to_run)) as executor:
        futures = {
            executor.submit(run_collector, name): name for name in sources_to_run
        }

        for future in as_completed(futures):
            source_name, result = future.result()
            results[source_name] = result
            total_packages += result["count"]
            status = "OK" if result["count"] > 0 else "FAILED"
            print(f"  {source_name}: {result['count']} packages [{status}]")

    # Print summary
    print(f"\n{'='*60}")
    print("SUMMARY")
    print(f"{'='*60}")
    for source_name, result in results.items():
        status = "OK" if result["count"] > 0 else "FAILED"
        print(f"  {source_name}: {result['count']} packages [{status}]")
        if result["errors"]:
            for error in result["errors"]:
                print(f"    - {error}")

    print(f"\nTotal: {total_packages} packages collected")
    return 0


if __name__ == "__main__":
    sys.exit(main())
