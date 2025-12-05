"""Flathub package collector with download statistics."""

from datetime import date
from typing import Optional

from collectors.base import BaseCollector, RateLimiter
from models import Package


class FlathubCollector(BaseCollector):
    """Collect CLI/terminal apps from Flathub with download stats."""

    source_name = "flathub"

    # Flathub API endpoints
    APPSTREAM_LIST_URL = "https://flathub.org/api/v2/appstream"
    APPSTREAM_APP_URL = "https://flathub.org/api/v2/appstream/{app_id}"
    STATS_URL = "https://flathub.org/api/v2/stats/{app_id}"

    # Known terminal emulators and CLI-related apps on Flathub
    KNOWN_CLI_APPS = [
        "org.gnome.Terminal",
        "org.gnome.Console",
        "org.kde.konsole",
        "com.raggesilver.BlackBox",
        "io.elementary.terminal",
        "org.wezfurlong.wezterm",
        "com.github.alecaddd.sequeler",
        "org.gnome.Builder",
        "com.visualstudio.code",
        "dev.zed.Zed",
        "io.neovim.nvim",
        "org.vim.Vim",
    ]

    def __init__(self):
        super().__init__()
        # Flathub has no documented rate limits, but be reasonable
        self.rate_limiter = RateLimiter(requests_per_hour=600)

    def collect(self, limit: Optional[int] = None) -> list[Package]:
        """Collect terminal/CLI apps from Flathub.

        Since Flathub is primarily for GUI apps and has limited CLI coverage,
        we focus on known terminal emulators and development tools.

        Args:
            limit: Optional limit on number of packages.

        Returns:
            List of Package objects with download stats.
        """
        print("Fetching Flathub app list...")

        try:
            response = self.session.get(self.APPSTREAM_LIST_URL, timeout=60)
            response.raise_for_status()
            all_app_ids = response.json()
        except Exception as e:
            self.errors.append(f"Failed to fetch Flathub app list: {e}")
            return []

        # Filter to CLI-related apps
        cli_keywords = ["terminal", "console", "shell", "cli", "command"]
        candidate_apps = []

        for app_id in all_app_ids:
            app_lower = app_id.lower()
            # Check if app ID contains CLI-related keywords
            if any(kw in app_lower for kw in cli_keywords):
                candidate_apps.append(app_id)
            elif app_id in self.KNOWN_CLI_APPS:
                candidate_apps.append(app_id)

        # Limit candidates to fetch
        if limit:
            candidate_apps = candidate_apps[:limit]

        print(f"Found {len(candidate_apps)} CLI-related apps, fetching details...")

        packages = []
        for app_id in candidate_apps:
            self.rate_limiter.wait()
            app_data = self._fetch_app_details(app_id)
            if not app_data:
                continue

            # Get stats
            stats = self._fetch_stats(app_id)

            name = app_data.get("name", app_id.split(".")[-1])
            packages.append(
                Package(
                    name=app_id.split(".")[-1].lower(),
                    display_name=name,
                    source=self.source_name,
                    source_id=app_id,
                    description=app_data.get("summary"),
                    homepage=app_data.get("projectUrl"),
                    popularity=stats.get("downloads_total") if stats else None,
                    collected_at=date.today(),
                )
            )

        print(f"Collected {len(packages)} CLI/terminal apps from Flathub")
        return packages

    def _fetch_app_details(self, app_id: str) -> Optional[dict]:
        """Fetch full metadata for an app.

        Args:
            app_id: Flathub app identifier.

        Returns:
            App metadata dict or None on error.
        """
        try:
            url = self.APPSTREAM_APP_URL.format(app_id=app_id)
            response = self.session.get(url, timeout=10)
            if response.status_code == 200:
                return response.json()
        except Exception:
            pass
        return None

    def _fetch_stats(self, app_id: str) -> Optional[dict]:
        """Fetch download statistics for an app.

        Args:
            app_id: Flathub app identifier.

        Returns:
            Stats dict or None on error.
        """
        try:
            url = self.STATS_URL.format(app_id=app_id)
            response = self.session.get(url, timeout=10)
            if response.status_code == 200:
                return response.json()
        except Exception:
            pass
        return None
