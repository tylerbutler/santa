#!/usr/bin/env python3
"""Generate cliff.toml configs from commit-types.json (single source of truth)."""

import json
import sys
from pathlib import Path

# Crate configurations: name, scope pattern for commit parsers, include unscoped commits
CRATE_CONFIGS = [
    {
        "name": "sickle",
        "path": "crates/sickle/cliff.toml",
        "tag_pattern": "sickle-v.*",
        "scope_pattern": "sickle",
        "include_unscoped": False,
    },
    {
        "name": "santa-data",
        "path": "crates/santa-data/cliff.toml",
        "tag_pattern": "santa-data-v.*",
        "scope_pattern": "santa-data",
        "include_unscoped": False,
    },
    {
        "name": "santa",
        "path": "crates/santa-cli/cliff.toml",
        "tag_pattern": "santa-v.*",
        "scope_pattern": "santa|santa-cli",
        "include_unscoped": True,
    },
]

CHANGELOG_TEMPLATE = '''# git-cliff config for {name}
# Auto-generated - edit generate-cliff-configs.py and commit-types.json instead

[changelog]
header = """# Changelog

All notable changes to this project will be documented in this file.
"""
body = """
{{% set visible_commits = commits | filter(attribute="group", value="_ignored") | length %}}\\
{{% set total_commits = commits | length %}}\\
{{% set has_visible_commits = visible_commits != total_commits %}}\\
{{% if version or has_visible_commits %}}\\
## {{% if version %}}[{{{{ version | trim_start_matches(pat="v") }}}}] - {{{{ timestamp | date(format="%Y-%m-%d") }}}}{{% else %}}[unreleased]{{% endif %}}
{{% if has_visible_commits %}}\\
{{% for group, group_commits in commits | group_by(attribute="group") %}}\\
{{% if group != "_ignored" %}}
### {{{{ group | upper_first }}}}
{{% for commit in group_commits %}}
- {{{{ commit.message | upper_first }}}}
{{% endfor %}}
{{% endif %}}\\
{{% endfor %}}\\
{{% else %}}
No notable changes in this release.

{{% endif %}}\\
{{% endif %}}\\

"""
trim = false

[git]
conventional_commits = true
filter_unconventional = true
tag_pattern = "{tag_pattern}"
commit_parsers = [
{commit_parsers}
]
'''


def generate_commit_parsers(
    types: dict, scope_pattern: str, include_unscoped: bool
) -> str:
    """Generate commit_parsers array entries from types config."""
    lines = []

    # Scoped commits
    for type_name, type_config in types.items():
        group = type_config.get("changelog_group")
        if group:
            lines.append(
                f'    {{ message = "^{type_name}\\\\({scope_pattern}\\\\)", group = "{group}" }},'
            )

    # Unscoped commits (for santa-cli which is the main package)
    if include_unscoped:
        for type_name, type_config in types.items():
            group = type_config.get("changelog_group")
            if group:
                lines.append(f'    {{ message = "^{type_name}:", group = "{group}" }},')

    # Catch-all for ignored commits
    lines.append('    { message = ".*", group = "_ignored" },')

    return "\n".join(lines)


def generate_cliff_config(crate_config: dict, types: dict) -> str:
    """Generate a complete cliff.toml for a crate."""
    commit_parsers = generate_commit_parsers(
        types, crate_config["scope_pattern"], crate_config["include_unscoped"]
    )

    return CHANGELOG_TEMPLATE.format(
        name=crate_config["name"],
        tag_pattern=crate_config["tag_pattern"],
        commit_parsers=commit_parsers,
    )


def main():
    root = Path(__file__).parent.parent
    config_path = root / "commit-types.json"

    with open(config_path) as f:
        config = json.load(f)

    types = config["types"]
    dry_run = "--dry-run" in sys.argv

    for crate_config in CRATE_CONFIGS:
        output_path = root / crate_config["path"]
        content = generate_cliff_config(crate_config, types)

        if dry_run:
            print(f"=== {output_path} ===")
            print(content)
        else:
            output_path.write_text(content)
            print(f"Wrote {output_path}")


if __name__ == "__main__":
    main()
