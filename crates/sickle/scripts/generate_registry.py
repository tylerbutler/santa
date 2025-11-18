#!/usr/bin/env python3
"""
Generate sickle CCL feature registry from test data.

This script extracts functions, behaviors, and coverage statistics
from test JSON files to create a dynamic, always-accurate registry.
"""

import json
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Set


@dataclass
class TestStats:
    """Statistics for a function or behavior."""

    total: int = 0
    examples: List[Dict] = None

    def __post_init__(self):
        if self.examples is None:
            self.examples = []


def load_test_suites(test_data_dir: Path) -> List[Dict]:
    """Load all test suite JSON files."""
    suites = []
    for json_file in test_data_dir.glob("*.json"):
        try:
            with open(json_file) as f:
                data = json.load(f)
                if "tests" in data:
                    suites.append({"file": json_file.name, "data": data})
        except json.JSONDecodeError as e:
            print(f"Warning: Failed to parse {json_file}: {e}", file=sys.stderr)
    return suites


def extract_metadata(suites: List[Dict]) -> tuple[Dict[str, TestStats], Dict[str, TestStats]]:
    """Extract function and behavior statistics from test suites."""
    functions: Dict[str, TestStats] = defaultdict(TestStats)
    behaviors: Dict[str, TestStats] = defaultdict(TestStats)

    for suite in suites:
        tests = suite["data"].get("tests", [])
        for test in tests:
            # Track functions
            for func in test.get("functions", []):
                stats = functions[func]
                stats.total += 1
                # Keep a few examples
                if len(stats.examples) < 3:
                    stats.examples.append(
                        {
                            "name": test.get("name"),
                            "input": test.get("input"),
                            "validation": test.get("validation"),
                            "source": suite["file"],
                        }
                    )

            # Track behaviors
            for behavior in test.get("behaviors", []):
                stats = behaviors[behavior]
                stats.total += 1
                if len(stats.examples) < 3:
                    stats.examples.append(
                        {
                            "name": test.get("name"),
                            "input": test.get("input"),
                            "validation": test.get("validation"),
                            "source": suite["file"],
                        }
                    )

    return dict(functions), dict(behaviors)


def generate_markdown(functions: Dict[str, TestStats], behaviors: Dict[str, TestStats]) -> str:
    """Generate markdown registry documentation."""
    lines = [
        "# Sickle CCL Feature Registry",
        "",
        "> **Auto-generated** from test data - DO NOT EDIT",
        "> ",
        "> Run `just sickle-registry` to regenerate",
        "",
        "This registry is extracted from the comprehensive test suite in `tests/test_data/*.json`.",
        "The test files are the source of truth for all supported features and behaviors.",
        "",
        "## Overview",
        "",
        f"- **Functions**: {len(functions)} distinct functions tested",
        f"- **Behaviors**: {len(behaviors)} distinct behaviors tested",
        f"- **Total test cases**: {sum(s.total for s in functions.values())}",
        "",
    ]

    # Functions section
    lines.extend(
        [
            "## Functions",
            "",
            "Core Model API methods and operations covered by the test suite:",
            "",
            "| Function | Test Cases | Description |",
            "|----------|------------|-------------|",
        ]
    )

    for func in sorted(functions.keys()):
        stats = functions[func]
        # Get validation type from first example to describe function
        validation = stats.examples[0]["validation"] if stats.examples else ""
        lines.append(f"| `{func}` | {stats.total} | Validation type: `{validation}` |")

    lines.extend(["", "### Function Examples", ""])

    for func in sorted(functions.keys()):
        stats = functions[func]
        lines.append(f"#### `{func}`")
        lines.append("")
        lines.append(f"**Test coverage**: {stats.total} test cases")
        lines.append("")

        if stats.examples:
            lines.append("**Example usage from tests**:")
            lines.append("")
            for ex in stats.examples[:2]:  # Show max 2 examples
                lines.append(f"- **{ex['name']}** (`{ex['source']}`)")
                if ex["input"]:
                    # Truncate long inputs
                    input_str = ex["input"][:100]
                    if len(ex["input"]) > 100:
                        input_str += "..."
                    lines.append(f"  ```ccl")
                    lines.append(f"  {input_str}")
                    lines.append(f"  ```")
            lines.append("")

    # Behaviors section
    lines.extend(
        [
            "## Behaviors",
            "",
            "Parser behaviors and configuration options tested by the suite:",
            "",
            "| Behavior | Test Cases |",
            "|----------|------------|",
        ]
    )

    for behavior in sorted(behaviors.keys()):
        stats = behaviors[behavior]
        lines.append(f"| `{behavior}` | {stats.total} |")

    lines.extend(["", "### Behavior Examples", ""])

    for behavior in sorted(behaviors.keys()):
        stats = behaviors[behavior]
        lines.append(f"#### `{behavior}`")
        lines.append("")
        lines.append(f"**Test coverage**: {stats.total} test cases")
        lines.append("")

        if stats.examples:
            lines.append("**Example usage from tests**:")
            lines.append("")
            for ex in stats.examples[:2]:
                lines.append(f"- **{ex['name']}** (`{ex['source']}`)")
                if ex["input"]:
                    input_str = ex["input"][:100]
                    if len(ex["input"]) > 100:
                        input_str += "..."
                    lines.append(f"  ```ccl")
                    lines.append(f"  {input_str}")
                    lines.append(f"  ```")
            lines.append("")

    # Usage note
    lines.extend(
        [
            "## Running Tests",
            "",
            "To run the comprehensive test suite:",
            "",
            "```bash",
            "just test-ccl",
            "```",
            "",
            "This runs all test cases from the JSON test data files and provides detailed",
            "pass/fail statistics per function and behavior.",
            "",
        ]
    )

    return "\n".join(lines)


def main():
    """Generate registry from test data."""
    # Determine paths
    script_dir = Path(__file__).parent
    test_data_dir = script_dir.parent / "tests" / "test_data"
    output_file = script_dir.parent / "REGISTRY.md"

    if not test_data_dir.exists():
        print(f"Error: Test data directory not found: {test_data_dir}", file=sys.stderr)
        sys.exit(1)

    # Load and process test data
    print(f"Loading test suites from {test_data_dir}...", file=sys.stderr)
    suites = load_test_suites(test_data_dir)
    print(f"Found {len(suites)} test suites", file=sys.stderr)

    print("Extracting metadata...", file=sys.stderr)
    functions, behaviors = extract_metadata(suites)
    print(f"Found {len(functions)} functions, {len(behaviors)} behaviors", file=sys.stderr)

    # Generate output
    print(f"Generating registry...", file=sys.stderr)
    markdown = generate_markdown(functions, behaviors)

    # Write output
    output_file.write_text(markdown)
    print(f"✓ Registry written to {output_file}", file=sys.stderr)


if __name__ == "__main__":
    main()
