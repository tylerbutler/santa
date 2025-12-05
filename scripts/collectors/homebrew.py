"""Homebrew package collector using analytics API."""

from datetime import date
from typing import Optional

from collectors.base import BaseCollector
from models import Package


class HomebrewCollector(BaseCollector):
    """Collect popular packages from Homebrew analytics."""

    source_name = "homebrew"

    # Analytics endpoint for install-on-request (user-requested installs, not deps)
    ANALYTICS_URL = (
        "https://formulae.brew.sh/api/analytics/install-on-request/365d.json"
    )

    def collect(self, limit: Optional[int] = 500) -> list[Package]:
        """Collect top packages from Homebrew analytics.

        Args:
            limit: Maximum number of packages to collect. Defaults to 500.

        Returns:
            List of Package objects with popularity data.
        """
        print(f"Fetching Homebrew analytics from {self.ANALYTICS_URL}...")

        try:
            response = self.session.get(self.ANALYTICS_URL, timeout=30)
            response.raise_for_status()
            data = response.json()
        except Exception as e:
            self.errors.append(f"Failed to fetch Homebrew analytics: {e}")
            return []

        items = data.get("items", [])
        if limit:
            items = items[:limit]

        packages = []
        for item in items:
            formula = item.get("formula", "")
            count_str = item.get("count", "0")
            rank = item.get("number", 0)

            # Parse count (may have commas)
            try:
                count = int(count_str.replace(",", ""))
            except ValueError:
                count = 0

            packages.append(
                Package(
                    name=formula.lower(),
                    display_name=formula,
                    source=self.source_name,
                    source_id=formula,
                    popularity=count,
                    popularity_rank=rank,
                    collected_at=date.today(),
                )
            )

        print(f"Collected {len(packages)} packages from Homebrew")
        return packages
