#!/usr/bin/env python3
"""Generate CCL format entries from verified packages.

Outputs CCL that can be appended to known_packages.ccl.
"""

import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional


# Sources that need name overrides (canonical_name -> {source: override_name})
NAME_OVERRIDES = {
    "github-cli": {"brew": "gh"},
    "git-delta": {"scoop": "delta"},
    "bottom": {"nix": "bottom-rs"},
    "golang": {"brew": "go"},
    "ripgrep": {"brew": "rg"},
}


def generate_ccl_entry(
    name: str,
    verified_sources: dict[str, str],
    min_sources: int = 2,
) -> Optional[str]:
    """Generate a CCL entry for a package.

    Args:
        name: Package name
        verified_sources: Dict of source -> package_name_in_source
        min_sources: Minimum sources required to include package

    Returns:
        CCL formatted string or None if not enough sources
    """
    if len(verified_sources) < min_sources:
        return None

    # Check if any sources need name overrides
    overrides = {}
    simple_sources = []

    for source, source_name in verified_sources.items():
        if source_name != name:
            # This source uses a different name
            overrides[source] = source_name
        else:
            simple_sources.append(source)

    # Generate CCL
    lines = [f"{name} ="]

    if overrides:
        # Complex format with overrides
        for source, override_name in overrides.items():
            lines.append(f"  {source} = {override_name}")

        if simple_sources:
            lines.append("  _sources =")
            for source in sorted(simple_sources):
                lines.append(f"    = {source}")
    else:
        # Simple format - all sources use same name
        for source in sorted(verified_sources.keys()):
            lines.append(f"  = {source}")

    return "\n".join(lines)


def load_existing_packages(ccl_path: Path) -> set[str]:
    """Load existing package names from known_packages.ccl."""
    if not ccl_path.exists():
        return set()

    existing = set()
    with open(ccl_path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("/=") or line.startswith("="):
                continue
            if "=" in line and not line.startswith(" "):
                name = line.split("=")[0].strip()
                if name:
                    existing.add(name.lower())

    return existing


def main():
    import argparse

    parser = argparse.ArgumentParser(
        description="Generate CCL entries from verified packages"
    )
    parser.add_argument(
        "--input",
        type=Path,
        default=Path(__file__).parent / "data" / "verified_packages.json",
        help="Input verified packages file",
    )
    parser.add_argument(
        "--ccl-path",
        type=Path,
        default=Path(__file__).parent.parent
        / "crates"
        / "santa-cli"
        / "data"
        / "known_packages.ccl",
        help="Path to existing known_packages.ccl (to skip existing)",
    )
    parser.add_argument(
        "--min-sources",
        type=int,
        default=2,
        help="Minimum sources required to include package (default: 2)",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=None,
        help="Limit number of packages to output",
    )
    parser.add_argument(
        "--append",
        action="store_true",
        help="Append to known_packages.ccl instead of stdout",
    )

    args = parser.parse_args()

    # Load verified packages
    with open(args.input) as f:
        data = json.load(f)

    packages = data.get("packages", [])

    # Load existing packages to skip
    existing = load_existing_packages(args.ccl_path)
    print(f"# Existing packages in CCL: {len(existing)}", file=sys.stderr)

    # Generate CCL entries
    entries = []
    skipped_existing = 0
    skipped_few_sources = 0

    for pkg in packages:
        name = pkg["name"]

        # Skip if already in CCL
        if name.lower() in existing:
            skipped_existing += 1
            continue

        verified_sources = pkg.get("verified_sources", {})

        entry = generate_ccl_entry(
            name=name,
            verified_sources=verified_sources,
            min_sources=args.min_sources,
        )

        if entry:
            entries.append(entry)
        else:
            skipped_few_sources += 1

        if args.limit and len(entries) >= args.limit:
            break

    print(f"# Skipped (already in CCL): {skipped_existing}", file=sys.stderr)
    print(f"# Skipped (< {args.min_sources} sources): {skipped_few_sources}", file=sys.stderr)
    print(f"# Generated entries: {len(entries)}", file=sys.stderr)

    # Output
    output = "\n\n".join(entries)

    if args.append:
        with open(args.ccl_path, "a") as f:
            f.write("\n\n/= Auto-generated entries\n\n")
            f.write(output)
            f.write("\n")
        print(f"# Appended to {args.ccl_path}", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
