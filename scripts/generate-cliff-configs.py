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


def generate_config(scope: str) -> str:
    parsers = ""
    for type_name, group_name in COMMIT_TYPES:
        parsers += f'    {{ message = "^{type_name}\\\\({scope}\\\\)", group = "{group_name}" }},\n'
    return TEMPLATE.format(name=scope, parsers=parsers)


def main():
    crates = [
        ("sickle", "crates/sickle"),
        ("santa-data", "crates/santa-data"),
        ("santa", "crates/santa"),
    ]

    root = Path(__file__).parent.parent

    for scope, crate_path in crates:
        config = generate_config(scope)
        out_path = root / crate_path / "cliff.toml"

        if "--dry-run" in sys.argv:
            print(f"=== {out_path} ===")
            print(config)
        else:
            out_path.write_text(config)
            print(f"Wrote {out_path}")


if __name__ == "__main__":
    main()
