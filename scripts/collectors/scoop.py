"""Scoop package collector from GitHub bucket."""

from datetime import date
from typing import Optional

from collectors.base import BaseCollector, RateLimiter, get_github_headers
from models import Package


class ScoopCollector(BaseCollector):
    """Collect packages from Scoop main bucket on GitHub."""

    source_name = "scoop"

    # GitHub API endpoint for main bucket contents
    BUCKET_API = "https://api.github.com/repos/ScoopInstaller/Main/contents/bucket"

    def __init__(self):
        super().__init__()
        # Rate limiter: 60/hr unauthenticated, 5000/hr with token
        self.rate_limiter = RateLimiter(requests_per_hour=50)

    def collect(self, limit: Optional[int] = None) -> list[Package]:
        """Collect packages from Scoop main bucket.

        Args:
            limit: Optional limit on number of packages.

        Returns:
            List of Package objects.
        """
        print(f"Fetching Scoop bucket contents from GitHub...")

        headers = get_github_headers()
        packages = []
        page = 1

        while True:
            self.rate_limiter.wait()

            try:
                response = self.session.get(
                    self.BUCKET_API,
                    params={"per_page": 100, "page": page},
                    headers=headers,
                    timeout=30,
                )
                response.raise_for_status()
                files = response.json()
            except Exception as e:
                self.errors.append(f"Failed to fetch bucket page {page}: {e}")
                break

            if not files:
                break

            for file_info in files:
                filename = file_info.get("name", "")
                if not filename.endswith(".json"):
                    continue

                # Extract package name from filename (e.g., "git.json" -> "git")
                pkg_name = filename[:-5]

                # Fetch individual manifest for description
                manifest = self._fetch_manifest(file_info.get("download_url", ""))

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

            print(f"  Page {page}: {len(files)} files, {len(packages)} packages total")
            page += 1

            # Safety limit to avoid runaway pagination
            if page > 50:
                self.errors.append("Exceeded max pages (50)")
                break

        print(f"Collected {len(packages)} packages from Scoop")
        return packages

    def _fetch_manifest(self, download_url: str) -> Optional[dict]:
        """Fetch and parse a Scoop manifest JSON.

        Args:
            download_url: URL to the raw manifest file.

        Returns:
            Parsed manifest dict or None on error.
        """
        if not download_url:
            return None

        self.rate_limiter.wait()

        try:
            response = self.session.get(download_url, timeout=10)
            response.raise_for_status()
            return response.json()
        except Exception:
            # Don't log every failed manifest fetch
            return None
