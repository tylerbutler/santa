"""Awesome CLI Apps collector from agarrharr/awesome-cli-apps."""

import re
from datetime import date
from typing import Optional

from collectors.base import BaseCollector
from models import Package


class AwesomeCliAppsCollector(BaseCollector):
    """Collect CLI tools from agarrharr/awesome-cli-apps curated list."""

    source_name = "awesome_cli_apps"

    README_URL = (
        "https://raw.githubusercontent.com/agarrharr/awesome-cli-apps/master/readme.md"
    )

    def collect(self, limit: Optional[int] = None) -> list[Package]:
        """Collect packages from awesome-cli-apps README.

        Args:
            limit: Optional limit on number of packages.

        Returns:
            List of Package objects.
        """
        print(f"Fetching awesome-cli-apps README from {self.README_URL}...")

        try:
            response = self.session.get(self.README_URL, timeout=30)
            response.raise_for_status()
            content = response.text
        except Exception as e:
            self.errors.append(f"Failed to fetch awesome-cli-apps README: {e}")
            return []

        packages = []
        current_category = None

        for line in content.splitlines():
            # Track current category from headers
            if line.startswith("## "):
                current_category = line[3:].strip()
                continue
            elif line.startswith("### "):
                # Subcategory - append to main category
                subcategory = line[4:].strip()
                if current_category:
                    current_category = f"{current_category} > {subcategory}"
                else:
                    current_category = subcategory
                continue

            # Parse list items: - [name](url) - description
            if line.startswith("- ["):
                match = re.match(r"- \[([^\]]+)\]\(([^)]+)\)(?: - (.*))?", line)
                if match:
                    name, url, description = match.groups()
                    name = name.strip()

                    packages.append(
                        Package(
                            name=name.lower(),
                            display_name=name,
                            source=self.source_name,
                            source_id=name,
                            description=description.strip() if description else None,
                            homepage=url.strip(),
                            category=current_category,
                            collected_at=date.today(),
                        )
                    )

                    if limit and len(packages) >= limit:
                        print(
                            f"Collected {len(packages)} packages from awesome-cli-apps (limit reached)"
                        )
                        return packages

        print(f"Collected {len(packages)} packages from awesome-cli-apps")
        return packages
