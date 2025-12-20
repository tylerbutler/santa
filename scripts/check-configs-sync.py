#!/usr/bin/env python3
"""Verify that generated configs match commit-types.json (single source of truth).

This script is used in CI to ensure configs don't drift out of sync.
Exit code 0 = in sync, exit code 1 = out of sync.
"""

import json
import subprocess
import sys
from pathlib import Path


def get_generated_content(script: str) -> str:
    """Run a generator script with --dry-run and capture output."""
    result = subprocess.run(
        ["python3", script, "--dry-run"],
        capture_output=True,
        text=True,
        check=True,
    )
    return result.stdout


def main():
    root = Path(__file__).parent.parent
    errors = []

    # Check commitlint config
    commitlint_path = root / ".commitlintrc.json"
    expected_commitlint = get_generated_content(root / "scripts" / "generate-commitlint-config.py")
    actual_commitlint = commitlint_path.read_text()

    if expected_commitlint.strip() != actual_commitlint.strip():
        errors.append(
            f".commitlintrc.json is out of sync with commit-types.json.\n"
            f"Run: python3 scripts/generate-commitlint-config.py"
        )

    # Check cliff configs
    cliff_script = root / "scripts" / "generate-cliff-configs.py"
    expected_output = get_generated_content(cliff_script)

    cliff_files = [
        root / "crates" / "sickle" / "cliff.toml",
        root / "crates" / "santa-data" / "cliff.toml",
        root / "crates" / "santa-cli" / "cliff.toml",
    ]

    for cliff_path in cliff_files:
        actual = cliff_path.read_text()
        # Extract the expected content for this file from the dry-run output
        marker = f"=== {cliff_path} ==="
        if marker in expected_output:
            start = expected_output.index(marker) + len(marker) + 1
            # Find the next marker or end
            next_marker = expected_output.find("=== /", start)
            expected = expected_output[start:next_marker].strip() if next_marker != -1 else expected_output[start:].strip()

            if expected != actual.strip():
                errors.append(
                    f"{cliff_path.relative_to(root)} is out of sync with commit-types.json.\n"
                    f"Run: python3 scripts/generate-cliff-configs.py"
                )

    if errors:
        print("Config sync check failed:\n")
        for error in errors:
            print(f"  - {error}\n")
        sys.exit(1)
    else:
        print("All configs are in sync with commit-types.json")
        sys.exit(0)


if __name__ == "__main__":
    main()
