"""Toolleeo CLI apps collector from CSV source."""

import csv
import io
from datetime import date
from typing import Optional

from collectors.base import BaseCollector
from models import Package


class ToolleeoCollector(BaseCollector):
    """Collect CLI tools from toolleeo/cli-apps CSV database."""

    source_name = "toolleeo"

    # Direct URL to CSV data file
    CSV_URL = "https://raw.githubusercontent.com/toolleeo/cli-apps/master/data/apps.csv"

    def collect(self, limit: Optional[int] = None) -> list[Package]:
        """Collect packages from toolleeo CSV.

        Args:
            limit: Optional limit on number of packages.

        Returns:
            List of Package objects with category metadata.
        """
        print(f"Fetching toolleeo CSV from {self.CSV_URL}...")

        try:
            response = self.session.get(self.CSV_URL, timeout=30)
            response.raise_for_status()
        except Exception as e:
            self.errors.append(f"Failed to fetch toolleeo CSV: {e}")
            return []

        packages = []
        reader = csv.DictReader(io.StringIO(response.text))

        for row in reader:
            name = row.get("name", "").strip()
            if not name:
                continue

            packages.append(
                Package(
                    name=name.lower(),
                    display_name=name,
                    source=self.source_name,
                    source_id=name,
                    description=row.get("description", "").strip() or None,
                    homepage=row.get("homepage", "").strip() or None,
                    git_url=row.get("git", "").strip() or None,
                    category=row.get("category", "").strip() or None,
                    collected_at=date.today(),
                )
            )

            if limit and len(packages) >= limit:
                break

        print(f"Collected {len(packages)} packages from toolleeo")
        return packages
