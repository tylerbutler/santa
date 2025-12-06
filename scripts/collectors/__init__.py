"""Package collectors for various sources."""

from collectors.base import BaseCollector
from collectors.homebrew import HomebrewCollector
from collectors.toolleeo import ToolleeoCollector
from collectors.modern_unix import ModernUnixCollector
from collectors.scoop import ScoopCollector
from collectors.aur import AURCollector
from collectors.arch import ArchCollector
from collectors.awesome_cli_apps import AwesomeCliAppsCollector

__all__ = [
    "BaseCollector",
    "HomebrewCollector",
    "ToolleeoCollector",
    "ModernUnixCollector",
    "ScoopCollector",
    "AURCollector",
    "ArchCollector",
    "AwesomeCliAppsCollector",
]
