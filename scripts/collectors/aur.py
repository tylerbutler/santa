"""AUR (Arch User Repository) package collector.

Uses the AUR metadata dump to get packages sorted by popularity.
"""

import gzip
import json
from datetime import date
from io import BytesIO
from typing import Optional

from collectors.base import BaseCollector
from models import Package


class AURCollector(BaseCollector):
    """Collect popular packages from the Arch User Repository."""

    source_name = "aur"

    # AUR provides a gzipped JSON dump of all package metadata
    METADATA_URL = "https://aur.archlinux.org/packages-meta-v1.json.gz"

    def collect(self, limit: Optional[int] = 500) -> list[Package]:
        """Collect top packages from AUR sorted by popularity.

        The AUR metadata dump includes a Popularity field that represents
        a time-weighted score based on user votes (each vote decays at 0.98/day).

        Args:
            limit: Maximum number of packages to collect. Defaults to 500.

        Returns:
            List of Package objects with popularity data.
        """
        print(f"Fetching AUR metadata from {self.METADATA_URL}...")

        try:
            response = self.session.get(self.METADATA_URL, timeout=120)
            response.raise_for_status()

            # Decompress gzipped response
            with gzip.GzipFile(fileobj=BytesIO(response.content)) as f:
                data = json.load(f)
        except Exception as e:
            self.errors.append(f"Failed to fetch AUR metadata: {e}")
            return []

        print(f"Loaded {len(data)} packages from AUR metadata")

        # Sort by popularity (descending)
        sorted_packages = sorted(
            data,
            key=lambda x: x.get("Popularity", 0) or 0,
            reverse=True,
        )

        if limit:
            sorted_packages = sorted_packages[:limit]

        packages = []
        for rank, pkg in enumerate(sorted_packages, start=1):
            name = pkg.get("Name", "")
            if not name:
                continue

            # Popularity is a float score with time decay
            popularity_score = pkg.get("Popularity", 0) or 0
            num_votes = pkg.get("NumVotes", 0) or 0

            packages.append(
                Package(
                    name=name.lower(),
                    display_name=name,
                    source=self.source_name,
                    source_id=name,
                    description=pkg.get("Description"),
                    homepage=pkg.get("URL"),
                    # Use NumVotes as the popularity metric (more intuitive than decay score)
                    popularity=num_votes,
                    popularity_rank=rank,
                    collected_at=date.today(),
                )
            )

        print(f"Collected {len(packages)} packages from AUR")
        return packages
