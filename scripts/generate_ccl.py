#!/usr/bin/env python3
"""Generate CCL format entries from verified packages.

Merges new packages with existing known_packages.ccl and outputs
alphabetically sorted CCL.
"""

import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


@dataclass
class PackageEntry:
    """Represents a package entry in CCL format."""

    name: str
    # Simple sources (same name as package)
    sources: list[str] = field(default_factory=list)
    # Source overrides (source -> override_name or complex config)
    overrides: dict[str, str] = field(default_factory=dict)
    # Complex overrides (source -> dict with pre/post/etc)
    complex_overrides: dict[str, dict] = field(default_factory=dict)

    def to_ccl(self) -> str:
        """Convert to CCL format string."""
        lines = [f"{self.name} ="]

        # Add complex overrides first
        for source, config in sorted(self.complex_overrides.items()):
            lines.append(f"  {source} =")
            for key, value in config.items():
                lines.append(f"    {key} = {value}")

        # Add simple overrides
        for source, override_name in sorted(self.overrides.items()):
            lines.append(f"  {source} = {override_name}")

        # Add _sources section if we have both overrides and simple sources
        if (self.overrides or self.complex_overrides) and self.sources:
            lines.append("  _sources =")
            for source in sorted(self.sources):
                lines.append(f"    = {source}")
        elif self.sources:
            # Simple format - just list sources
            for source in sorted(self.sources):
                lines.append(f"  = {source}")

        return "\n".join(lines)


def parse_existing_ccl(ccl_path: Path) -> dict[str, PackageEntry]:
    """Parse existing known_packages.ccl into PackageEntry objects."""
    if not ccl_path.exists():
        return {}

    packages: dict[str, PackageEntry] = {}

    with open(ccl_path) as f:
        content = f.read()

    # Split into package blocks
    # Each package starts with "name =" at the beginning of a line
    blocks = re.split(r"\n(?=[a-zA-Z@_][^\n]*\s*=\s*$)", content, flags=re.MULTILINE)

    for block in blocks:
        block = block.strip()
        if not block or block.startswith("/="):
            continue

        lines = block.split("\n")
        if not lines:
            continue

        # First line should be "name ="
        first_line = lines[0].strip()
        if "=" not in first_line:
            continue

        name = first_line.split("=")[0].strip()
        if not name:
            continue

        entry = PackageEntry(name=name)

        # Parse the rest of the block
        i = 1
        in_sources = False
        current_override_source = None

        while i < len(lines):
            line = lines[i]
            stripped = line.strip()

            if not stripped or stripped.startswith("/="):
                i += 1
                continue

            # Check indentation level
            indent = len(line) - len(line.lstrip())

            if stripped == "_sources =":
                in_sources = True
                current_override_source = None
                i += 1
                continue

            if stripped.startswith("= "):
                # Simple source
                source = stripped[2:].strip()
                entry.sources.append(source)
                i += 1
                continue

            if "=" in stripped:
                parts = stripped.split("=", 1)
                key = parts[0].strip()
                value = parts[1].strip() if len(parts) > 1 else ""

                if indent == 2:  # Top-level override
                    in_sources = False
                    if value:
                        # Simple override: source = override_name
                        entry.overrides[key] = value
                        current_override_source = None
                    else:
                        # Complex override start: source =
                        current_override_source = key
                        entry.complex_overrides[key] = {}
                elif indent == 4 and current_override_source:
                    # Inside complex override
                    entry.complex_overrides[current_override_source][key] = value

            i += 1

        if entry.sources or entry.overrides or entry.complex_overrides:
            packages[name.lower()] = entry

    return packages


def merge_packages(
    existing: dict[str, PackageEntry],
    verified_path: Path,
    min_sources: int = 1,
) -> dict[str, PackageEntry]:
    """Merge verified packages with existing entries."""

    with open(verified_path) as f:
        data = json.load(f)

    new_packages = data.get("packages", [])
    merged = dict(existing)  # Start with existing

    for pkg in new_packages:
        name = pkg["name"]
        name_lower = name.lower()
        verified_sources = pkg.get("verified_sources", {})

        if len(verified_sources) < min_sources:
            continue

        if name_lower in merged:
            # Update existing entry with new sources
            entry = merged[name_lower]
            for source, source_name in verified_sources.items():
                if source_name == name:
                    if source not in entry.sources:
                        entry.sources.append(source)
                else:
                    # Override needed
                    if source not in entry.overrides:
                        entry.overrides[source] = source_name
        else:
            # Create new entry
            entry = PackageEntry(name=name)
            for source, source_name in verified_sources.items():
                if source_name == name:
                    entry.sources.append(source)
                else:
                    entry.overrides[source] = source_name

            merged[name_lower] = entry

    return merged


def generate_ccl_output(packages: dict[str, PackageEntry]) -> str:
    """Generate complete CCL output, alphabetically sorted."""

    # Separate simple and complex entries
    simple_entries = []
    complex_entries = []

    for entry in packages.values():
        if entry.overrides or entry.complex_overrides:
            complex_entries.append(entry)
        else:
            simple_entries.append(entry)

    # Sort each list alphabetically by name
    simple_entries.sort(key=lambda e: e.name.lower())
    complex_entries.sort(key=lambda e: e.name.lower())

    lines = []

    # Simple entries first
    if simple_entries:
        lines.append("/= Packages with simple format (no source-specific overrides)")
        for entry in simple_entries:
            lines.append(entry.to_ccl())
            lines.append("")

    # Complex entries
    if complex_entries:
        lines.append("/= Packages with complex format (have source-specific overrides)")
        lines.append("")
        for entry in complex_entries:
            lines.append(entry.to_ccl())
            lines.append("")

    return "\n".join(lines)


def main():
    import argparse

    parser = argparse.ArgumentParser(
        description="Generate merged CCL from verified packages"
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
        help="Path to known_packages.ccl",
    )
    parser.add_argument(
        "--min-sources",
        type=int,
        default=1,
        help="Minimum sources required to include package (default: 1)",
    )
    parser.add_argument(
        "--write",
        action="store_true",
        help="Write output to CCL file instead of stdout",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be written without writing",
    )

    args = parser.parse_args()

    # Parse existing CCL
    print(f"# Parsing existing CCL from {args.ccl_path}", file=sys.stderr)
    existing = parse_existing_ccl(args.ccl_path)
    print(f"# Found {len(existing)} existing packages", file=sys.stderr)

    # Merge with verified packages
    print(f"# Merging with verified packages from {args.input}", file=sys.stderr)
    merged = merge_packages(existing, args.input, args.min_sources)
    new_count = len(merged) - len(existing)
    print(f"# Total packages after merge: {len(merged)} (+{new_count} new)", file=sys.stderr)

    # Generate output
    output = generate_ccl_output(merged)

    if args.dry_run:
        print("# Dry run - would write the following:", file=sys.stderr)
        print(output)
    elif args.write:
        with open(args.ccl_path, "w") as f:
            f.write(output)
        print(f"# Written to {args.ccl_path}", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
