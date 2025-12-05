"""Base collector class and utilities."""

import json
import os
import time
from abc import ABC, abstractmethod
from datetime import date
from pathlib import Path
from typing import Optional

import requests
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry

from models import CollectionResult, Package


class RateLimiter:
    """Simple rate limiter for API requests."""

    def __init__(self, requests_per_hour: int = 60):
        self.delay = 3600 / requests_per_hour
        self.last_request = 0.0

    def wait(self):
        """Wait if necessary to respect rate limit."""
        elapsed = time.time() - self.last_request
        if elapsed < self.delay:
            time.sleep(self.delay - elapsed)
        self.last_request = time.time()


def get_session(retries: int = 3) -> requests.Session:
    """Create a requests session with retry logic."""
    session = requests.Session()
    retry = Retry(
        total=retries,
        backoff_factor=0.5,
        status_forcelist=[429, 500, 502, 503, 504],
    )
    adapter = HTTPAdapter(max_retries=retry)
    session.mount("http://", adapter)
    session.mount("https://", adapter)
    return session


def get_github_headers() -> dict:
    """Get headers for GitHub API requests."""
    headers = {"Accept": "application/vnd.github.v3+json"}
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"token {token}"
    return headers


class BaseCollector(ABC):
    """Abstract base class for package collectors."""

    source_name: str = "unknown"
    output_dir: Path = Path(__file__).parent.parent / "data" / "raw"

    def __init__(self):
        self.session = get_session()
        self.errors: list[str] = []

    @abstractmethod
    def collect(self, limit: Optional[int] = None) -> list[Package]:
        """Collect packages from this source.

        Args:
            limit: Optional limit on number of packages to collect.

        Returns:
            List of Package objects.
        """
        pass

    def save(self, packages: list[Package]) -> Path:
        """Save collected packages to JSON file.

        Args:
            packages: List of packages to save.

        Returns:
            Path to the saved file.
        """
        self.output_dir.mkdir(parents=True, exist_ok=True)
        output_path = self.output_dir / f"{self.source_name}.json"

        result = CollectionResult(
            source=self.source_name,
            packages=packages,
            collected_at=date.today(),
            total_count=len(packages),
            errors=self.errors,
        )

        with open(output_path, "w") as f:
            json.dump(result.model_dump(mode="json"), f, indent=2)

        return output_path

    def run(self, limit: Optional[int] = None) -> tuple[list[Package], Path]:
        """Run collection and save results.

        Args:
            limit: Optional limit on number of packages.

        Returns:
            Tuple of (packages, output_path).
        """
        packages = self.collect(limit=limit)
        output_path = self.save(packages)
        return packages, output_path
