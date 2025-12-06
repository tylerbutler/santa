#!/usr/bin/env python3
"""Verify package availability across package managers.

Uses already-collected data from package manager sources (homebrew, scoop, arch, aur)
to verify package availability without making additional HTTP requests.
"""

import json
import sys
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path
from typing import Optional


@dataclass
class VerifiedPackage:
    """A package with verified source availability."""

    name: str
    display_name: str
    score: int
    description: Optional[str] = None

    # Verified sources: source_name -> package_name_in_source
    verified_sources: dict[str, str] = field(default_factory=dict)


def load_package_index(data_dir: Path) -> dict[str, dict[str, set[str]]]:
    """Load package names from collected JSON files.

    Returns a dict mapping source -> set of normalized package names.
    """
    index: dict[str, set[str]] = {}

    source_files = {
        "brew": "homebrew.json",
        "scoop": "scoop.json",
        "arch": "arch.json",
        "aur": "aur.json",
    }

    for source, filename in source_files.items():
        filepath = data_dir / filename
        if not filepath.exists():
            continue

        with open(filepath) as f:
            data = json.load(f)

        names = set()
        for pkg in data.get("packages", []):
            name = pkg.get("name", "").lower()
            if name:
                names.add(name)

        index[source] = names
        print(f"Loaded {len(names)} packages from {source}")

    return index


# Static lists for package managers we don't collect from
KNOWN_APT_PACKAGES = {
    "git", "curl", "wget", "vim", "neovim", "tmux", "htop", "tree",
    "jq", "ripgrep", "fd-find", "bat", "fzf", "zsh", "fish",
    "docker", "docker-compose", "pandoc", "ffmpeg", "imagemagick",
    "python3", "nodejs", "golang", "rustc", "cmake", "gcc", "make",
    "gnupg", "openssh-client", "rsync", "screen", "ncdu", "tig",
}

KNOWN_PACMAN_PACKAGES = {
    "git", "curl", "wget", "vim", "neovim", "tmux", "htop", "tree",
    "jq", "ripgrep", "fd", "bat", "fzf", "zsh", "fish", "exa",
    "docker", "docker-compose", "pandoc", "ffmpeg", "imagemagick",
    "python", "nodejs", "go", "rust", "cmake", "gcc", "make",
    "gnupg", "openssh", "rsync", "screen", "ncdu", "tig", "lazygit",
    "bottom", "dust", "procs", "sd", "hyperfine", "tokei", "starship",
}

KNOWN_NIX_PACKAGES = {
    "git", "curl", "wget", "vim", "neovim", "tmux", "htop", "tree",
    "jq", "ripgrep", "fd", "bat", "fzf", "zsh", "fish", "eza",
    "docker", "docker-compose", "pandoc", "ffmpeg", "imagemagick",
    "python3", "nodejs", "go", "rustc", "cmake", "gcc", "gnumake",
    "gnupg", "openssh", "rsync", "screen", "ncdu", "tig", "lazygit",
    "bottom", "dust", "procs", "sd", "hyperfine", "tokei", "starship",
    "direnv", "zoxide", "atuin", "delta", "difftastic",
}


def verify_packages(
    crossref_path: Path,
    output_path: Path,
    data_dir: Path,
    limit: int = 100,
) -> list[VerifiedPackage]:
    """Verify package availability using collected data."""

    # Load package index from collected data
    print(f"Loading package index from {data_dir}...")
    pkg_index = load_package_index(data_dir)

    with open(crossref_path) as f:
        data = json.load(f)

    packages = data.get("packages", [])[:limit]
    print(f"\nVerifying {len(packages)} packages...")

    verified = []
    for i, pkg in enumerate(packages, 1):
        name = pkg["name"].lower()

        vp = VerifiedPackage(
            name=pkg["name"],
            display_name=pkg.get("display_name", pkg["name"]),
            score=pkg.get("score", 0),
            description=pkg.get("description"),
        )

        # Check against collected package manager data
        for source, names in pkg_index.items():
            if name in names:
                vp.verified_sources[source] = name

        # Check static lists for other managers
        if name in KNOWN_APT_PACKAGES or name.replace("-", "") in KNOWN_APT_PACKAGES:
            vp.verified_sources["apt"] = name
        if name in KNOWN_PACMAN_PACKAGES:
            vp.verified_sources["pacman"] = name
        if name in KNOWN_NIX_PACKAGES:
            vp.verified_sources["nix"] = name

        sources_str = ", ".join(sorted(vp.verified_sources.keys())) or "none"
        print(f"[{i}/{len(packages)}] {vp.name} [{sources_str}]")

        verified.append(vp)

    # Save results
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w") as f:
        json.dump(
            {
                "generated_at": date.today().isoformat(),
                "total_verified": len(verified),
                "packages": [
                    {
                        "name": vp.name,
                        "display_name": vp.display_name,
                        "score": vp.score,
                        "description": vp.description,
                        "verified_sources": vp.verified_sources,
                    }
                    for vp in verified
                ],
            },
            f,
            indent=2,
        )

    print(f"\nResults saved to {output_path}")
    return verified


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Verify package availability")
    parser.add_argument(
        "--input",
        type=Path,
        default=Path(__file__).parent / "data" / "crossref_results.json",
        help="Input cross-reference results file",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).parent / "data" / "verified_packages.json",
        help="Output verified packages file",
    )
    parser.add_argument(
        "--data-dir",
        type=Path,
        default=Path(__file__).parent / "data" / "raw",
        help="Directory containing collected package data",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=100,
        help="Number of packages to verify (default: 100)",
    )

    args = parser.parse_args()

    verified = verify_packages(
        crossref_path=args.input,
        output_path=args.output,
        data_dir=args.data_dir,
        limit=args.limit,
    )

    # Summary
    print("\n" + "=" * 60)
    print("VERIFICATION SUMMARY")
    print("=" * 60)

    with_brew = sum(1 for v in verified if "brew" in v.verified_sources)
    with_scoop = sum(1 for v in verified if "scoop" in v.verified_sources)
    with_apt = sum(1 for v in verified if "apt" in v.verified_sources)
    with_pacman = sum(1 for v in verified if "pacman" in v.verified_sources)
    with_nix = sum(1 for v in verified if "nix" in v.verified_sources)
    with_multiple = sum(1 for v in verified if len(v.verified_sources) >= 2)

    print(f"Total packages verified: {len(verified)}")
    print(f"Available in brew:       {with_brew}")
    print(f"Available in scoop:      {with_scoop}")
    print(f"Available in apt:        {with_apt}")
    print(f"Available in pacman:     {with_pacman}")
    print(f"Available in nix:        {with_nix}")
    print(f"Available in 2+ sources: {with_multiple}")


if __name__ == "__main__":
    main()
