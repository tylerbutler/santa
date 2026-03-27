---
name: changelog-entry
description: Create a changie changelog fragment for this repo. Use whenever a code change needs a changelog entry — after finishing a feature or bugfix, when the user asks to add a changelog entry, or when a PR is missing one. Also use this if the finishing-a-development-branch workflow needs a changelog entry added.
---

# Creating a Changelog Entry

This repo uses [changie](https://changie.dev) for changelog management. Changelog entries are YAML fragment files in `.changes/unreleased/`.

## Steps

1. **Read `.changie.yaml`** to get the current list of projects and kinds. Do not hardcode these — the config is the source of truth and may change over time.

2. **Determine the project** by looking at which crate(s) your changes touch. Map the crate directory to the project key:
   - Changes in `crates/santa-cli/` → project key from config
   - Changes in `crates/santa-data/` → project key from config
   - Changes in `crates/sickle/` → project key from config
   - Changes in `crates/sickle-cli/` → project key from config
   - If changes span multiple crates, create one fragment per project.

3. **Determine the kind** based on what the change does. The kind labels in `.changie.yaml` map to semver bumps (`auto` field), so pick the right one — `Added` for new features, `Fixed` for bugfixes, etc.

4. **Write the fragment file** with this exact format:

   ```yaml
   project: <project-key>
   kind: <Kind>
   body: <One-line description of the change>
   time: <ISO 8601 timestamp with timezone>
   ```

   - The `project` field is **required** — omitting it breaks `changie batch` in multi-project repos.
   - The `body` should be a concise, user-facing description. Reference the issue number if there is one, e.g. `Fix widget crash on empty input (#45)`.
   - The `time` field uses full ISO 8601 with microseconds and timezone offset (e.g. `2026-03-26T12:00:00.000000-07:00`).

5. **Name the file** following the pattern: `{project}-{Kind}-{YYYYMMDD}-{HHMMSS}.yaml`
   - Example: `sickle-Fixed-20260326-120000.yaml`
   - Place it in `.changes/unreleased/`

6. **Validate** by running `changie batch auto --dry-run --project <project-key>` and confirming the entry appears in the output without errors.
