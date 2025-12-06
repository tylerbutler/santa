"""Scoop package collector from GitHub bucket.

Uses shallow git clone for fast local access to all manifests.
"""

import json
import shutil
import subprocess
from datetime import date
from pathlib import Path
from typing import Optional

from collectors.base import BaseCollector
from models import Package


class ScoopCollector(BaseCollector):
    """Collect packages from Scoop main bucket using local git clone."""

    source_name = "scoop"

    # Scoop main bucket repository
    BUCKET_REPO = "https://github.com/ScoopInstaller/Main.git"

    # Local cache directory for cloned repo
    CACHE_DIR = Path(__file__).parent.parent / "data" / ".cache" / "scoop-main"

    def collect(self, limit: Optional[int] = None) -> list[Package]:
        """Collect packages from Scoop main bucket.

        Uses a shallow git clone for fast access to all manifests.
        The clone is cached and updated on subsequent runs.

        Args:
            limit: Optional limit on number of packages.

        Returns:
            List of Package objects.
        """
        bucket_dir = self._ensure_bucket()
        if not bucket_dir:
            return []

        manifest_dir = bucket_dir / "bucket"
        if not manifest_dir.exists():
            self.errors.append(f"Bucket directory not found: {manifest_dir}")
            return []

        print(f"Reading Scoop manifests from {manifest_dir}...")

        packages = []
        manifest_files = sorted(manifest_dir.glob("*.json"))

        for manifest_path in manifest_files:
            pkg_name = manifest_path.stem

            manifest = self._parse_manifest(manifest_path)

            packages.append(
                Package(
                    name=pkg_name.lower(),
                    display_name=pkg_name,
                    source=self.source_name,
                    source_id=pkg_name,
                    description=manifest.get("description") if manifest else None,
                    homepage=manifest.get("homepage") if manifest else None,
                    collected_at=date.today(),
                )
            )

            if limit and len(packages) >= limit:
                print(f"Collected {len(packages)} packages from Scoop (limit reached)")
                return packages

        print(f"Collected {len(packages)} packages from Scoop")
        return packages

    def _ensure_bucket(self) -> Optional[Path]:
        """Ensure the Scoop bucket is cloned and up-to-date.

        Uses shallow clone with depth=1 for speed.

        Returns:
            Path to cloned bucket directory, or None on error.
        """
        self.CACHE_DIR.parent.mkdir(parents=True, exist_ok=True)

        if self.CACHE_DIR.exists():
            # Update existing clone
            print("Updating Scoop bucket cache...")
            try:
                subprocess.run(
                    ["git", "pull", "--depth=1", "--ff-only"],
                    cwd=self.CACHE_DIR,
                    check=True,
                    capture_output=True,
                    timeout=120,
                )
                return self.CACHE_DIR
            except subprocess.CalledProcessError as e:
                # Pull failed, try fresh clone
                print(f"Pull failed, re-cloning: {e.stderr.decode()[:100]}")
                shutil.rmtree(self.CACHE_DIR, ignore_errors=True)
            except subprocess.TimeoutExpired:
                self.errors.append("Git pull timed out")
                shutil.rmtree(self.CACHE_DIR, ignore_errors=True)

        # Fresh shallow clone
        print("Cloning Scoop main bucket (shallow)...")
        try:
            subprocess.run(
                ["git", "clone", "--depth=1", self.BUCKET_REPO, str(self.CACHE_DIR)],
                check=True,
                capture_output=True,
                timeout=180,
            )
            return self.CACHE_DIR
        except subprocess.CalledProcessError as e:
            self.errors.append(f"Failed to clone Scoop bucket: {e.stderr.decode()[:200]}")
            return None
        except subprocess.TimeoutExpired:
            self.errors.append("Git clone timed out")
            return None

    def _parse_manifest(self, path: Path) -> Optional[dict]:
        """Parse a Scoop manifest JSON file.

        Args:
            path: Path to the manifest file.

        Returns:
            Parsed manifest dict or None on error.
        """
        try:
            with open(path, encoding="utf-8") as f:
                return json.load(f)
        except (json.JSONDecodeError, OSError):
            return None
