#!/usr/bin/env python3
"""Generate git-cliff config files for each crate with scope-based filtering."""

import sys
from pathlib import Path

COMMIT_TYPES = [
    ("feat", "Features"),
    ("fix", "Bug Fixes"),
    ("docs", "Documentation"),
    ("refactor", "Refactoring"),
    ("perf", "Performance"),
    ("test", "Testing"),
    ("chore", "Miscellaneous"),
    ("ci", "CI"),
]

TEMPLATE = '''# git-cliff config for {name}
# Auto-generated - edit generate-cliff-configs.py instead

[changelog]
header = """# Changelog

All notable changes to this project will be documented in this file.
"""
body = """
{{% for group, commits in commits | group_by(attribute="group") %}}
### {{{{ group | upper_first }}}}
{{% for commit in commits %}}
- {{{{ commit.message | upper_first }}}}
{{% endfor %}}
{{% endfor %}}
"""
trim = true

[git]
conventional_commits = true
filter_unconventional = true
commit_parsers = [
{parsers}    {{ message = ".*", skip = true }},
]
'''


def generate_config(name: str, scopes: list[str]) -> str:
    parsers = ""
    # Build alternation pattern for multiple scopes: (santa|santa-cli)
    scope_pattern = "|".join(scopes) if len(scopes) > 1 else scopes[0]
    for type_name, group_name in COMMIT_TYPES:
        # Match scoped commits for this crate
        parsers += f'    {{ message = "^{type_name}\\\\({scope_pattern}\\\\)", group = "{group_name}" }},\n'
    for type_name, group_name in COMMIT_TYPES:
        # Match unscoped commits (apply to all crates)
        parsers += f'    {{ message = "^{type_name}:", group = "{group_name}" }},\n'
    return TEMPLATE.format(name=name, parsers=parsers)


def main():
    # (name, path, scopes) - scopes are conventional commit scopes to include
    crates = [
        ("sickle", "crates/sickle", ["sickle"]),
        ("santa-data", "crates/santa-data", ["santa-data"]),
        ("santa", "crates/santa-cli", ["santa", "santa-cli"]),
    ]

    root = Path(__file__).parent.parent

    for name, crate_path, scopes in crates:
        config = generate_config(name, scopes)
        out_path = root / crate_path / "cliff.toml"

        if "--dry-run" in sys.argv:
            print(f"=== {out_path} ===")
            print(config)
        else:
            out_path.write_text(config)
            print(f"Wrote {out_path}")


if __name__ == "__main__":
    main()
