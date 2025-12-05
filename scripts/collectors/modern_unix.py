"""Modern Unix collector - modern alternatives to classic Unix tools."""

from datetime import date
from typing import Optional

from bs4 import BeautifulSoup

from collectors.base import BaseCollector
from models import Package


class ModernUnixCollector(BaseCollector):
    """Collect modern Unix tool alternatives from ibraheemdev/modern-unix."""

    source_name = "modern_unix"

    README_URL = (
        "https://raw.githubusercontent.com/ibraheemdev/modern-unix/master/README.md"
    )

    def collect(self, limit: Optional[int] = None) -> list[Package]:
        """Collect packages from modern-unix README.

        Args:
            limit: Optional limit on number of packages.

        Returns:
            List of Package objects.
        """
        print(f"Fetching modern-unix README from {self.README_URL}...")

        try:
            response = self.session.get(self.README_URL, timeout=30)
            response.raise_for_status()
            content = response.text
        except Exception as e:
            self.errors.append(f"Failed to fetch modern-unix README: {e}")
            return []

        packages = []
        soup = BeautifulSoup(content, "html.parser")

        # Find all h1 tags with code links (tool entries)
        for h1 in soup.find_all("h1"):
            code_tag = h1.find("code")
            if not code_tag:
                continue

            link = h1.find("a")
            if not link:
                continue

            name = code_tag.get_text().strip()
            url = link.get("href", "")

            # Find the description in the next p tag
            description = ""
            next_p = h1.find_next("p")
            if next_p and next_p.get("align") == "center":
                description = next_p.get_text().strip()

            if name:
                packages.append(
                    Package(
                        name=name.lower(),
                        display_name=name,
                        source=self.source_name,
                        source_id=name,
                        description=description or None,
                        homepage=url,
                        collected_at=date.today(),
                    )
                )

                if limit and len(packages) >= limit:
                    break

        print(f"Collected {len(packages)} packages from modern-unix")
        return packages
