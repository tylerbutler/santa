"""Arch Linux official repository package collector.

Uses the pkgstats.archlinux.de API to get packages sorted by usage popularity.
"""

from datetime import date
from typing import Optional

from collectors.base import BaseCollector
from models import Package


class ArchCollector(BaseCollector):
    """Collect popular packages from Arch Linux official repositories.

    Uses pkgstats.archlinux.de which tracks package installation statistics
    from users who have opted in to share their package data.
    """

    source_name = "arch"

    # pkgstats API endpoint
    API_URL = "https://pkgstats.archlinux.de/api/packages"

    def collect(self, limit: Optional[int] = 200) -> list[Package]:
        """Collect top packages from Arch Linux official repositories.

        The pkgstats API returns packages sorted by popularity based on
        actual installation statistics from contributing users.

        Args:
            limit: Maximum number of packages to collect. Defaults to 200.

        Returns:
            List of Package objects with popularity data.
        """
        print(f"Fetching Arch package statistics from {self.API_URL}...")

        packages = []
        offset = 0
        batch_size = 500  # API supports up to 500 per request

        while len(packages) < (limit or float("inf")):

            try:
                response = self.session.get(
                    self.API_URL,
                    params={
                        "limit": batch_size,
                        "offset": offset,
                    },
                    headers={"Accept": "application/json"},
                    timeout=30,
                )
                response.raise_for_status()
                data = response.json()
            except Exception as e:
                self.errors.append(f"Failed to fetch Arch stats at offset {offset}: {e}")
                break

            pkg_list = data.get("packagePopularities", [])
            if not pkg_list:
                break

            for pkg in pkg_list:
                name = pkg.get("name", "")
                if not name:
                    continue

                # Skip AUR packages (they appear in pkgstats too)
                # Official packages don't have certain prefixes/patterns
                # pkgstats includes all packages, so we can't easily filter
                # But the top packages are typically official ones

                # Use count (actual install count) rather than percentage
                # since count is more comparable to other sources
                install_count = pkg.get("count", 0) or 0

                packages.append(
                    Package(
                        name=name.lower(),
                        display_name=name,
                        source=self.source_name,
                        source_id=name,
                        popularity=install_count,
                        popularity_rank=len(packages) + 1,
                        collected_at=date.today(),
                    )
                )

                if limit and len(packages) >= limit:
                    break

            offset += batch_size

            # Check if we've reached the end
            total = data.get("total", 0)
            if offset >= total:
                break

            print(f"  Fetched {len(packages)} packages so far...")

        print(f"Collected {len(packages)} packages from Arch repositories")
        return packages
