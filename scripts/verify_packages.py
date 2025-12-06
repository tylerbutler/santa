#!/usr/bin/env python3
"""Verify package availability across package managers.

Takes cross-referenced packages and verifies which package managers
actually have each package available, including any name variations.
"""

import json
import sys
import time
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path
from typing import Optional

import requests
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry

# Known name mappings: canonical name -> {source: actual_name}
NAME_MAPPINGS = {
    "ripgrep": {"brew": "ripgrep", "scoop": "ripgrep"},  # brew formula is ripgrep, not rg
    "git-delta": {"brew": "git-delta", "scoop": "delta"},
    "github-cli": {"brew": "gh", "scoop": "gh"},
    "fd-find": {"brew": "fd", "scoop": "fd"},
    "neovim": {"brew": "neovim", "scoop": "neovim"},
    "bottom": {"brew": "bottom", "scoop": "bottom", "nix": "bottom-rs"},
    "node": {"brew": "node", "scoop": "nodejs"},
    "golang": {"brew": "go", "scoop": "go"},
    "python": {"brew": "python@3.13", "scoop": "python"},
}


def get_session() -> requests.Session:
    """Create a requests session with retry logic."""
    session = requests.Session()
    retry = Retry(total=3, backoff_factor=0.5, status_forcelist=[429, 500, 502, 503, 504])
    adapter = HTTPAdapter(max_retries=retry)
    session.mount("http://", adapter)
    session.mount("https://", adapter)
    return session


@dataclass
class VerifiedPackage:
    """A package with verified source availability."""

    name: str
    display_name: str
    score: int
    description: Optional[str] = None

    # Verified sources: source_name -> package_name_in_source
    verified_sources: dict[str, str] = field(default_factory=dict)

    # Sources that were checked but package not found
    not_found_in: list[str] = field(default_factory=list)


class BrewVerifier:
    """Verify packages in Homebrew."""

    API_URL = "https://formulae.brew.sh/api/formula/{name}.json"

    def __init__(self, session: requests.Session):
        self.session = session
        self._cache: dict[str, bool] = {}

    def verify(self, name: str) -> Optional[str]:
        """Check if a package exists in Homebrew.

        Returns the formula name if found, None otherwise.
        """
        # Check name mappings first
        if name in NAME_MAPPINGS and "brew" in NAME_MAPPINGS[name]:
            check_name = NAME_MAPPINGS[name]["brew"]
        else:
            check_name = name

        if check_name in self._cache:
            return check_name if self._cache[check_name] else None

        try:
            url = self.API_URL.format(name=check_name)
            response = self.session.get(url, timeout=10)
            if response.status_code == 200:
                self._cache[check_name] = True
                return check_name
        except Exception:
            pass

        self._cache[check_name] = False
        return None


class ScoopVerifier:
    """Verify packages in Scoop main bucket."""

    BUCKET_URL = "https://raw.githubusercontent.com/ScoopInstaller/Main/master/bucket/{name}.json"

    def __init__(self, session: requests.Session):
        self.session = session
        self._cache: dict[str, bool] = {}

    def verify(self, name: str) -> Optional[str]:
        """Check if a package exists in Scoop main bucket.

        Returns the manifest name if found, None otherwise.
        """
        # Check name mappings first
        if name in NAME_MAPPINGS and "scoop" in NAME_MAPPINGS[name]:
            check_name = NAME_MAPPINGS[name]["scoop"]
        else:
            check_name = name

        if check_name in self._cache:
            return check_name if self._cache[check_name] else None

        try:
            url = self.BUCKET_URL.format(name=check_name)
            response = self.session.get(url, timeout=10)
            if response.status_code == 200:
                self._cache[check_name] = True
                return check_name
        except Exception:
            pass

        self._cache[check_name] = False
        return None


# Static lists for package managers that are harder to query
# These are commonly available CLI tools
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
    limit: int = 100,
    verify_brew: bool = True,
    verify_scoop: bool = True,
) -> list[VerifiedPackage]:
    """Verify package availability across sources."""

    with open(crossref_path) as f:
        data = json.load(f)

    packages = data.get("packages", [])[:limit]
    print(f"Verifying {len(packages)} packages...")

    session = get_session()
    brew_verifier = BrewVerifier(session) if verify_brew else None
    scoop_verifier = ScoopVerifier(session) if verify_scoop else None

    verified = []
    for i, pkg in enumerate(packages, 1):
        name = pkg["name"]
        print(f"[{i}/{len(packages)}] Verifying {name}...", end=" ")

        vp = VerifiedPackage(
            name=name,
            display_name=pkg.get("display_name", name),
            score=pkg.get("score", 0),
            description=pkg.get("description"),
        )

        # Verify Homebrew
        if brew_verifier:
            brew_name = brew_verifier.verify(name)
            if brew_name:
                vp.verified_sources["brew"] = brew_name
            else:
                vp.not_found_in.append("brew")
            time.sleep(0.1)  # Rate limit

        # Verify Scoop
        if scoop_verifier:
            scoop_name = scoop_verifier.verify(name)
            if scoop_name:
                vp.verified_sources["scoop"] = scoop_name
            else:
                vp.not_found_in.append("scoop")
            time.sleep(0.1)  # Rate limit

        # Check static lists for other managers
        if name in KNOWN_APT_PACKAGES or name.replace("-", "") in KNOWN_APT_PACKAGES:
            vp.verified_sources["apt"] = name
        if name in KNOWN_PACMAN_PACKAGES:
            vp.verified_sources["pacman"] = name
        if name in KNOWN_NIX_PACKAGES:
            vp.verified_sources["nix"] = name

        sources_str = ", ".join(vp.verified_sources.keys()) or "none"
        print(f"[{sources_str}]")

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
                        "not_found_in": vp.not_found_in,
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
        "--limit",
        type=int,
        default=100,
        help="Number of packages to verify (default: 100)",
    )
    parser.add_argument(
        "--skip-brew",
        action="store_true",
        help="Skip Homebrew verification (faster)",
    )
    parser.add_argument(
        "--skip-scoop",
        action="store_true",
        help="Skip Scoop verification (faster)",
    )

    args = parser.parse_args()

    verified = verify_packages(
        crossref_path=args.input,
        output_path=args.output,
        limit=args.limit,
        verify_brew=not args.skip_brew,
        verify_scoop=not args.skip_scoop,
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
