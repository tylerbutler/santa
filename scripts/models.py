"""Unified data models for package collection."""

from datetime import date
from typing import Optional

from pydantic import BaseModel, Field


class Package(BaseModel):
    """Unified package representation for all sources."""

    name: str = Field(description="Normalized lowercase package name")
    display_name: Optional[str] = Field(
        default=None, description="Original display name from source"
    )
    source: str = Field(description="Source identifier (e.g., 'homebrew', 'scoop')")
    source_id: str = Field(description="Package identifier in the source system")

    # Popularity metrics (null for curated-only sources)
    popularity: Optional[int] = Field(
        default=None, description="Install/download count if available"
    )
    popularity_rank: Optional[int] = Field(
        default=None, description="Rank within source (1 = most popular)"
    )

    # Metadata
    description: Optional[str] = Field(default=None, description="Package description")
    homepage: Optional[str] = Field(default=None, description="Project homepage URL")
    git_url: Optional[str] = Field(default=None, description="Git repository URL")
    category: Optional[str] = Field(
        default=None, description="Category from curated sources"
    )

    # Collection metadata
    collected_at: date = Field(
        default_factory=date.today, description="Date when this data was collected"
    )

    class Config:
        json_encoders = {date: lambda v: v.isoformat()}


class CollectionResult(BaseModel):
    """Result of a collection run."""

    source: str = Field(description="Source identifier")
    packages: list[Package] = Field(description="Collected packages")
    collected_at: date = Field(default_factory=date.today)
    total_count: int = Field(description="Total packages collected")
    errors: list[str] = Field(default_factory=list, description="Any errors encountered")
