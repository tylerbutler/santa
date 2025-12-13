# Command-Line Help for `santa`

This document contains the help content for the `santa` command-line program.

**Command Overview:**

* [`santa`↴](#santa)
* [`santa status`↴](#santa-status)
* [`santa install`↴](#santa-install)
* [`santa add`↴](#santa-add)
* [`santa config`↴](#santa-config)
* [`santa completions`↴](#santa-completions)
* [`santa sources`↴](#santa-sources)
* [`santa sources update`↴](#santa-sources-update)
* [`santa sources list`↴](#santa-sources-list)
* [`santa sources show`↴](#santa-sources-show)
* [`santa sources clear`↴](#santa-sources-clear)

## `santa`

a tool that manages packages across different platforms

**Usage:** `santa [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `status` — Prints the status of santa packages
* `install` — Installs packages
* `add` — Adds a package to the tracking list for a package source
* `config` — 
* `completions` — Generate shell completions
* `sources` — Manage package sources

###### **Options:**

* `-b`, `--builtin-only` — Load ONLY the default config
* `-v`, `--verbose` — Increase logging level
* `-x`, `--execute` — Enable dangerous direct execution mode (default: safe script generation)
* `--format <FORMAT>` — Script format for safe mode (auto-detects based on platform)

  Default value: `auto`

  Possible values:
  - `auto`:
    Auto-detect based on platform (PowerShell on Windows, Shell elsewhere)
  - `shell`:
    Force shell script (.sh) - Unix/Linux/macOS
  - `power-shell`:
    Force PowerShell script (.ps1) - Windows

* `--output-dir <OUTPUT_DIR>` — Output directory for generated scripts



## `santa status`

Prints the status of santa packages

**Usage:** `santa status [OPTIONS]`

###### **Options:**

* `-a`, `--all` — List all packages, not just missing ones



## `santa install`

Installs packages

**Usage:** `santa install [SOURCE]`

###### **Arguments:**

* `<SOURCE>`



## `santa add`

Adds a package to the tracking list for a package source

**Usage:** `santa add [PACKAGE] [SOURCE]`

###### **Arguments:**

* `<PACKAGE>`
* `<SOURCE>`



## `santa config`

**Usage:** `santa config [OPTIONS]`

###### **Options:**

* `-p`, `--packages` — Show full config
* `--pipe`



## `santa completions`

Generate shell completions

**Usage:** `santa completions <SHELL>`

###### **Arguments:**

* `<SHELL>` — Shell to generate completions for

  Possible values: `bash`, `elvish`, `fish`, `powershell`, `zsh`




## `santa sources`

Manage package sources

**Usage:** `santa sources <COMMAND>`

###### **Subcommands:**

* `update` — Download the latest source definitions from GitHub
* `list` — List all available sources (from all layers)
* `show` — Show details about a specific source
* `clear` — Remove downloaded sources (revert to bundled only)



## `santa sources update`

Download the latest source definitions from GitHub

**Usage:** `santa sources update`



## `santa sources list`

List all available sources (from all layers)

**Usage:** `santa sources list [OPTIONS]`

###### **Options:**

* `--origin <ORIGIN>` — Show only sources from a specific origin (bundled, downloaded, custom)



## `santa sources show`

Show details about a specific source

**Usage:** `santa sources show <NAME>`

###### **Arguments:**

* `<NAME>` — Name of the source to show



## `santa sources clear`

Remove downloaded sources (revert to bundled only)

**Usage:** `santa sources clear`



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>

