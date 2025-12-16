#!/usr/bin/env python3
"""Generate git-cliff config files for each crate with scope-based filtering.

Reads commit types from commit-types.json (single source of truth).
"""

import json
import sys
from pathlib import Path


def load_commit_types(root: Path) -> dict:
    """Load commit types from the shared config file."""
    config_path = root / "commit-types.json"
    with open(config_path) as f:
        return json.load(f)


def get_changelog_types(config: dict) -> list[tuple[str, str]]:
    """Extract types that should appear in changelogs (have a changelog_group)."""
    return [
        (type_name, type_info["changelog_group"])
        for type_name, type_info in config["types"].items()
        if type_info.get("changelog_group")
    ]


TEMPLATE = '''# git-cliff config for {name}
# Auto-generated - edit generate-cliff-configs.py and commit-types.json instead

[changelog]
header = """# Changelog

All notable changes to this project will be documented in this file.
"""
body = """
## {{% if version %}}[{{{{ version | trim_start_matches(pat="v") }}}}] - {{{{ timestamp | date(format="%Y-%m-%d") }}}}{{% else %}}[unreleased]{{% endif %}}
{{% set visible_commits = commits | filter(attribute="group", value="_ignored") | length %}}\\
{{% set total_commits = commits | length %}}\\
{{% if visible_commits == total_commits %}}
No notable changes in this release.
{{% else %}}\\
{{% for group, group_commits in commits | group_by(attribute="group") %}}\\
{{% if group != "_ignored" %}}
### {{{{ group | upper_first }}}}
{{% for commit in group_commits %}}
- {{{{ commit.message | upper_first }}}}
{{% endfor %}}
{{% endif %}}\\
{{% endfor %}}\\
{{% endif %}}
"""
trim = false

[git]
conventional_commits = true
filter_unconventional = true
tag_pattern = "{tag_pattern}"
commit_parsers = [
{parsers}    {{ message = ".*", group = "_ignored" }},
]
'''


def generate_config(
    name: str,
    scopes: list[str],
    tag_pattern: str,
    commit_types: list[tuple[str, str]],
    include_unscoped: bool = True,
) -> str:
    parsers = ""
    # Build alternation pattern for multiple scopes: (santa|santa-cli)
    scope_pattern = "|".join(scopes) if len(scopes) > 1 else scopes[0]
    for type_name, group_name in commit_types:
        # Match scoped commits for this crate
        parsers += f'    {{ message = "^{type_name}\\\\({scope_pattern}\\\\)", group = "{group_name}" }},\n'
    if include_unscoped:
        for type_name, group_name in commit_types:
            # Match unscoped commits (apply to all crates)
            parsers += f'    {{ message = "^{type_name}:", group = "{group_name}" }},\n'
    return TEMPLATE.format(name=name, parsers=parsers, tag_pattern=tag_pattern)


def main():
    # (name, path, scopes, tag_pattern, include_unscoped) - scopes are conventional commit scopes to include
    # tag_pattern is a regex to match only tags for this crate
    # include_unscoped: whether to include unscoped commits (e.g., "feat:" without a scope)
    crates = [
        ("sickle", "crates/sickle", ["sickle"], "sickle-v.*", False),
        ("santa-data", "crates/santa-data", ["santa-data"], "santa-data-v.*", False),
        ("santa", "crates/santa-cli", ["santa", "santa-cli"], "santa-v.*", True),
    ]

    root = Path(__file__).parent.parent

    # Load commit types from shared config
    config = load_commit_types(root)
    commit_types = get_changelog_types(config)

    for name, crate_path, scopes, tag_pattern, include_unscoped in crates:
        cliff_config = generate_config(name, scopes, tag_pattern, commit_types, include_unscoped)
        out_path = root / crate_path / "cliff.toml"

        if "--dry-run" in sys.argv:
            print(f"=== {out_path} ===")
            print(cliff_config)
        else:
            out_path.write_text(cliff_config)
            print(f"Wrote {out_path}")


if __name__ == "__main__":
    main()
