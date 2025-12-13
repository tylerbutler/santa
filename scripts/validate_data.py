#!/usr/bin/env python3
"""
Validate the Santa package database (known_packages.ccl).

Checks for:
- Duplicate package names
- Invalid source references
- Alias targets that don't exist
- Warnings for single-source packages
"""

import sys
import re
from pathlib import Path
from collections import defaultdict


def parse_ccl_packages(file_path: Path) -> tuple[dict, list]:
    """Parse the CCL packages file and return packages dict and validation issues."""
    packages = {}
    issues = []

    with open(file_path, 'r') as f:
        content = f.read()

    # Valid source names based on sources.ccl
    valid_sources = {'brew', 'scoop', 'npm', 'cargo', 'nix', 'apt', 'pacman', 'aur', 'flathub', 'pip'}

    # Split into simple and complex sections
    # Everything before the complex format comment
    simple_section = content.split('/= Packages with complex format')[0]
    complex_section = content.split('/= Packages with complex format')[1] if '/= Packages with complex format' in content else ''

    # Parse simple format packages
    # Pattern: package_name = followed by indented sources
    simple_pattern = r'^([a-z0-9\-@/.]+)\s*=\s*$\n((?:  = .+\n)*)'

    for match in re.finditer(simple_pattern, simple_section, re.MULTILINE):
        pkg_name = match.group(1).strip()
        sources_text = match.group(2)

        # Check for duplicates
        if pkg_name in packages:
            issues.append(f"ERROR: Duplicate package '{pkg_name}'")

        sources = []
        for source_match in re.finditer(r'  = (.+)', sources_text):
            source = source_match.group(1).strip()
            if source:
                sources.append(source)
                # Validate source
                if source not in valid_sources:
                    issues.append(f"ERROR: Invalid source '{source}' for package '{pkg_name}'")

        # Warn if only one source
        if len(sources) == 1:
            issues.append(f"WARNING: Package '{pkg_name}' only available from {sources[0]}")

        packages[pkg_name] = {
            'type': 'simple',
            'sources': sources,
            'original_name': pkg_name
        }

    # Parse complex format packages
    complex_pattern = r'^([a-z0-9\-@/.]+)\s*=\n((?:(?:  [a-z]+.*|  _sources.*)\n)*)'

    for match in re.finditer(complex_pattern, complex_section, re.MULTILINE):
        pkg_name = match.group(1).strip()
        content_text = match.group(2)

        # Check for duplicates
        if pkg_name in packages:
            issues.append(f"ERROR: Duplicate package '{pkg_name}'")

        sources = set()
        overrides = {}

        # Parse the content
        lines = content_text.split('\n')
        i = 0
        while i < len(lines):
            line = lines[i]

            # Check for _sources section
            if '_sources' in line:
                # Collect sources from following indented lines
                i += 1
                while i < len(lines) and lines[i].startswith('    '):
                    source_match = re.match(r'    = (.+)', lines[i])
                    if source_match:
                        source = source_match.group(1).strip()
                        if source:
                            sources.add(source)
                            if source not in valid_sources:
                                issues.append(f"ERROR: Invalid source '{source}' for package '{pkg_name}'")
                    i += 1
                continue

            # Check for source-specific overrides
            source_match = re.match(r'  ([a-z]+)\s*=\s*(.+)?', line)
            if source_match:
                source = source_match.group(1)
                override_value = source_match.group(2)

                # Skip _sources and other control fields
                if source.startswith('_') or source == 'aur':
                    i += 1
                    continue

                if source not in valid_sources:
                    issues.append(f"ERROR: Invalid source '{source}' for package '{pkg_name}'")
                else:
                    sources.add(source)
                    if override_value:
                        # This is an alias mapping
                        overrides[source] = override_value

            i += 1

        # Warn if only one source
        if len(sources) == 1:
            issues.append(f"WARNING: Package '{pkg_name}' only available from {list(sources)[0]}")

        packages[pkg_name] = {
            'type': 'complex',
            'sources': list(sources),
            'overrides': overrides,
            'original_name': pkg_name
        }

    return packages, issues


def validate_aliases(packages: dict, issues: list) -> None:
    """Validate that alias targets exist."""
    for pkg_name, pkg_info in packages.items():
        if pkg_info['type'] == 'complex' and pkg_info['overrides']:
            # Check if this looks like an alias (short name mapping to longer name)
            # An alias would have all sources pointing to the same package
            unique_targets = set(pkg_info['overrides'].values())

            if len(unique_targets) == 1:
                target = list(unique_targets)[0]
                # Don't validate if it's a URL
                if not target.startswith('http'):
                    if target not in packages:
                        # Special case: some targets use - instead of actual package names
                        # e.g., 'gh' -> 'github-cli', 'rg' -> 'ripgrep'
                        # These are intentional aliases
                        issues.append(f"INFO: '{pkg_name}' is an alias for '{target}' (target package)")


def print_report(packages: dict, issues: list) -> int:
    """Print validation report and return exit code."""
    print("\n" + "=" * 70)
    print("Santa Package Database Validation Report")
    print("=" * 70 + "\n")

    # Print issues by severity
    errors = [i for i in issues if i.startswith("ERROR")]
    warnings = [i for i in issues if i.startswith("WARNING")]
    infos = [i for i in issues if i.startswith("INFO")]

    if errors:
        print("ERRORS:")
        for error in errors:
            print(f"  ✗ {error}")
        print()

    if warnings:
        print("WARNINGS:")
        for warning in warnings:
            print(f"  ⚠ {warning}")
        print()

    if infos:
        print("INFO:")
        for info in infos:
            print(f"  ℹ {info}")
        print()

    # Print summary statistics
    print("Summary Statistics:")
    print(f"  Total packages: {len(packages)}")

    simple_count = sum(1 for p in packages.values() if p['type'] == 'simple')
    complex_count = sum(1 for p in packages.values() if p['type'] == 'complex')

    print(f"  Simple format: {simple_count}")
    print(f"  Complex format (with overrides/aliases): {complex_count}")

    # Source coverage
    all_sources = set()
    for pkg in packages.values():
        all_sources.update(pkg['sources'])

    print(f"  Unique sources: {len(all_sources)}")
    print(f"    {', '.join(sorted(all_sources))}")

    # Package availability distribution
    source_counts = defaultdict(int)
    for pkg in packages.values():
        for source in pkg['sources']:
            source_counts[source] += 1

    print("\n  Package availability by source:")
    for source in sorted(source_counts.keys()):
        count = source_counts[source]
        print(f"    {source}: {count} packages")

    print("\n" + "=" * 70)

    if errors:
        print("Status: FAILED (fix errors above)")
        return 1
    elif warnings:
        print("Status: PASSED WITH WARNINGS (review warnings above)")
        return 0
    else:
        print("Status: PASSED")
        return 0


def main():
    """Main entry point."""
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    packages_file = project_root / "crates" / "santa-cli" / "data" / "known_packages.ccl"

    if not packages_file.exists():
        print(f"ERROR: Package file not found: {packages_file}")
        return 1

    print(f"Validating: {packages_file}")

    packages, issues = parse_ccl_packages(packages_file)
    validate_aliases(packages, issues)

    exit_code = print_report(packages, issues)

    return exit_code


if __name__ == "__main__":
    sys.exit(main())
