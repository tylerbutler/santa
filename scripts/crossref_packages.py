#!/usr/bin/env python3
"""Cross-reference packages across collected sources and score by popularity.

Builds a unified index of packages from all collected data sources,
scores them by popularity and curation presence, and outputs ranked results.
"""

import json
import re
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path
from typing import Optional

# Add scripts directory to path
sys.path.insert(0, str(Path(__file__).parent))

from models import Package


@dataclass
class CrossRefPackage:
    """A package with cross-referenced data from multiple sources."""

    name: str  # Canonical normalized name
    display_name: str = ""
    description: Optional[str] = None
    homepage: Optional[str] = None

    # Source presence
    in_homebrew: bool = False
    in_toolleeo: bool = False
    in_modern_unix: bool = False
    in_awesome_cli: bool = False

    # Homebrew specific (has popularity data)
    homebrew_rank: Optional[int] = None
    homebrew_installs: Optional[int] = None

    # Category from curated sources
    category: Optional[str] = None

    # Computed score
    score: int = 0

    # All source names where this package appears
    sources: list[str] = field(default_factory=list)


def normalize_name(name: str) -> str:
    """Normalize a package name for matching.

    - Lowercase
    - Strip common suffixes (-cli, -bin, -git, -rs)
    - Handle @ scoped packages
    """
    name = name.lower().strip()

    # Handle npm scoped packages - use the package name part
    if name.startswith("@") and "/" in name:
        name = name.split("/")[-1]

    # Strip common suffixes
    suffixes = ["-cli", "-bin", "-git", "-rs", "-go", "-rust"]
    for suffix in suffixes:
        if name.endswith(suffix):
            name = name[: -len(suffix)]
            break

    # Handle version suffixes like python@3.13
    if "@" in name:
        name = name.split("@")[0]

    return name


def load_json_packages(filepath: Path) -> list[Package]:
    """Load packages from a collected JSON file."""
    if not filepath.exists():
        print(f"Warning: {filepath} not found")
        return []

    with open(filepath) as f:
        data = json.load(f)

    packages = []
    for pkg_data in data.get("packages", []):
        packages.append(Package(**pkg_data))

    return packages


def build_crossref_index(data_dir: Path) -> dict[str, CrossRefPackage]:
    """Build a cross-referenced index from all source files."""
    index: dict[str, CrossRefPackage] = {}

    # Load Homebrew (has popularity data)
    homebrew_pkgs = load_json_packages(data_dir / "homebrew.json")
    print(f"Loaded {len(homebrew_pkgs)} packages from Homebrew")

    for pkg in homebrew_pkgs:
        norm_name = normalize_name(pkg.name)
        if norm_name not in index:
            index[norm_name] = CrossRefPackage(
                name=norm_name,
                display_name=pkg.display_name or pkg.name,
                description=pkg.description,
                homepage=pkg.homepage,
            )

        index[norm_name].in_homebrew = True
        index[norm_name].homebrew_rank = pkg.popularity_rank
        index[norm_name].homebrew_installs = pkg.popularity
        index[norm_name].sources.append("homebrew")

        # Update display name and description if better
        if pkg.display_name and not index[norm_name].display_name:
            index[norm_name].display_name = pkg.display_name
        if pkg.description and not index[norm_name].description:
            index[norm_name].description = pkg.description

    # Load Modern Unix (high quality curated)
    modern_unix_pkgs = load_json_packages(data_dir / "modern_unix.json")
    print(f"Loaded {len(modern_unix_pkgs)} packages from modern-unix")

    for pkg in modern_unix_pkgs:
        norm_name = normalize_name(pkg.name)
        if norm_name not in index:
            index[norm_name] = CrossRefPackage(
                name=norm_name,
                display_name=pkg.display_name or pkg.name,
                description=pkg.description,
                homepage=pkg.homepage,
            )

        index[norm_name].in_modern_unix = True
        index[norm_name].sources.append("modern_unix")

        if pkg.description and not index[norm_name].description:
            index[norm_name].description = pkg.description
        if pkg.homepage and not index[norm_name].homepage:
            index[norm_name].homepage = pkg.homepage

    # Load Toolleeo (large curated list with categories)
    toolleeo_pkgs = load_json_packages(data_dir / "toolleeo.json")
    print(f"Loaded {len(toolleeo_pkgs)} packages from toolleeo")

    for pkg in toolleeo_pkgs:
        norm_name = normalize_name(pkg.name)
        if norm_name not in index:
            index[norm_name] = CrossRefPackage(
                name=norm_name,
                display_name=pkg.display_name or pkg.name,
                description=pkg.description,
                homepage=pkg.homepage,
            )

        index[norm_name].in_toolleeo = True
        index[norm_name].sources.append("toolleeo")

        if pkg.category and not index[norm_name].category:
            index[norm_name].category = pkg.category
        if pkg.description and not index[norm_name].description:
            index[norm_name].description = pkg.description

    # Load Awesome CLI Apps
    awesome_pkgs = load_json_packages(data_dir / "awesome_cli_apps.json")
    print(f"Loaded {len(awesome_pkgs)} packages from awesome-cli-apps")

    for pkg in awesome_pkgs:
        norm_name = normalize_name(pkg.name)
        if norm_name not in index:
            index[norm_name] = CrossRefPackage(
                name=norm_name,
                display_name=pkg.display_name or pkg.name,
                description=pkg.description,
                homepage=pkg.homepage,
            )

        index[norm_name].in_awesome_cli = True
        index[norm_name].sources.append("awesome_cli_apps")

        if pkg.category and not index[norm_name].category:
            index[norm_name].category = pkg.category
        if pkg.description and not index[norm_name].description:
            index[norm_name].description = pkg.description

    return index


def score_packages(index: dict[str, CrossRefPackage]) -> None:
    """Calculate scores for all packages in the index."""
    for pkg in index.values():
        score = 0

        # Homebrew rank (most weight - has real usage data)
        # Top 500 packages get 500-1 points based on rank
        if pkg.homebrew_rank:
            score += max(0, 501 - pkg.homebrew_rank)

        # Curated list presence (quality signals)
        if pkg.in_modern_unix:
            score += 200  # High quality modern CLI replacements
        if pkg.in_toolleeo:
            score += 50  # Large curated CLI list
        if pkg.in_awesome_cli:
            score += 50  # Community curated

        # Bonus for appearing in multiple sources
        source_count = len(set(pkg.sources))
        if source_count >= 3:
            score += 100
        elif source_count >= 2:
            score += 50

        pkg.score = score


def load_existing_packages(ccl_path: Path) -> set[str]:
    """Load existing package names from known_packages.ccl."""
    if not ccl_path.exists():
        return set()

    existing = set()
    with open(ccl_path) as f:
        for line in f:
            line = line.strip()
            # Skip comments and empty lines
            if not line or line.startswith("/=") or line.startswith("="):
                continue
            # Package definitions start with name followed by =
            if "=" in line and not line.startswith(" "):
                name = line.split("=")[0].strip()
                if name:
                    existing.add(normalize_name(name))

    return existing


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Cross-reference collected packages")
    parser.add_argument(
        "--data-dir",
        type=Path,
        default=Path(__file__).parent / "data" / "raw",
        help="Directory containing collected JSON files",
    )
    parser.add_argument(
        "--ccl-path",
        type=Path,
        default=Path(__file__).parent.parent
        / "crates"
        / "santa-cli"
        / "data"
        / "known_packages.ccl",
        help="Path to existing known_packages.ccl",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).parent / "data" / "crossref_results.json",
        help="Output file for cross-reference results",
    )
    parser.add_argument(
        "--top",
        type=int,
        default=200,
        help="Number of top packages to output (default: 200)",
    )
    parser.add_argument(
        "--include-existing",
        action="store_true",
        help="Include packages already in known_packages.ccl",
    )

    args = parser.parse_args()

    # Build cross-reference index
    print(f"\nBuilding cross-reference index from {args.data_dir}...")
    index = build_crossref_index(args.data_dir)
    print(f"Total unique packages indexed: {len(index)}")

    # Score packages
    print("\nScoring packages...")
    score_packages(index)

    # Load existing packages
    existing = load_existing_packages(args.ccl_path)
    print(f"Existing packages in CCL: {len(existing)}")

    # Sort by score and filter
    ranked = sorted(index.values(), key=lambda p: p.score, reverse=True)

    if not args.include_existing:
        ranked = [p for p in ranked if p.name not in existing]
        print(f"After filtering existing: {len(ranked)} candidates")

    # Take top N
    top_packages = ranked[: args.top]

    # Output results
    print(f"\nTop {len(top_packages)} packages by score:")
    print("-" * 80)

    results = []
    for i, pkg in enumerate(top_packages, 1):
        sources_str = ", ".join(sorted(set(pkg.sources)))
        print(f"{i:3}. {pkg.name:<25} score={pkg.score:4} sources=[{sources_str}]")

        results.append(
            {
                "rank": i,
                "name": pkg.name,
                "display_name": pkg.display_name,
                "description": pkg.description,
                "homepage": pkg.homepage,
                "score": pkg.score,
                "in_homebrew": pkg.in_homebrew,
                "in_modern_unix": pkg.in_modern_unix,
                "in_toolleeo": pkg.in_toolleeo,
                "in_awesome_cli": pkg.in_awesome_cli,
                "homebrew_rank": pkg.homebrew_rank,
                "homebrew_installs": pkg.homebrew_installs,
                "category": pkg.category,
                "sources": list(set(pkg.sources)),
            }
        )

    # Save results
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with open(args.output, "w") as f:
        json.dump(
            {
                "generated_at": date.today().isoformat(),
                "total_indexed": len(index),
                "existing_in_ccl": len(existing),
                "packages": results,
            },
            f,
            indent=2,
        )

    print(f"\nResults saved to {args.output}")

    # Summary stats
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)
    in_homebrew = sum(1 for p in top_packages if p.in_homebrew)
    in_modern = sum(1 for p in top_packages if p.in_modern_unix)
    in_toolleeo = sum(1 for p in top_packages if p.in_toolleeo)
    in_awesome = sum(1 for p in top_packages if p.in_awesome_cli)

    print(f"Packages in Homebrew:      {in_homebrew}")
    print(f"Packages in modern-unix:   {in_modern}")
    print(f"Packages in toolleeo:      {in_toolleeo}")
    print(f"Packages in awesome-cli:   {in_awesome}")


if __name__ == "__main__":
    main()
