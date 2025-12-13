#!/usr/bin/env python3
"""
Package Collection Script for Santa

Collects CLI tools from various sources and checks their availability across
different package managers. Outputs both JSON (for analysis) and source-organized
CCL files (for santa package data).

IMPORTANT: All CCL reading/writing is isolated in the section marked with
=========== CCL Functions ===========
This allows easy replacement with proper sickle library integration later.
"""

import requests
import re
import json
import subprocess
import os
from bs4 import BeautifulSoup
from tabulate import tabulate

tools = {}
available_managers = {}

# ============================================================================
# CCL Writing Functions (TODO: Replace with proper sickle library integration)
# ============================================================================

def write_ccl_file(filepath, manager_name, packages):
    """
    Write a CCL file for a specific package manager.

    Args:
        filepath: Path to write the CCL file
        manager_name: Name of the package manager (e.g., 'brew', 'scoop')
        packages: List of dicts with 'name' and optional 'description' keys

    TODO: Replace this simple string-based implementation with proper sickle
          library calls once we have CCL generation support.
    """
    with open(filepath, "w") as f:
        # Write header comments
        f.write(f"/= {manager_name.capitalize()} packages\n")
        f.write(f"/= Auto-generated from package collection script\n")
        f.write(f"/= Found {len(packages)} installable packages\n")
        f.write("\n")

        # Deduplicate and sort by name
        seen = set()
        unique_packages = []
        for pkg in packages:
            name = pkg['name'] if isinstance(pkg, dict) else pkg
            if name not in seen:
                seen.add(name)
                unique_packages.append(pkg)
        unique_packages.sort(key=lambda p: p['name'] if isinstance(p, dict) else p)

        # Write packages
        for pkg in unique_packages:
            if isinstance(pkg, dict):
                name = pkg['name']
                desc = pkg.get('description', '')
                if desc:
                    # Package with description - use complex format
                    f.write(f"{name} =\n")
                    f.write(f"  _description = {desc}\n")
                else:
                    # Simple format
                    f.write(f"{name} =\n")
            else:
                # Just a name string
                f.write(f"{pkg} =\n")

# ============================================================================
# End CCL Writing Functions
# ============================================================================

# Check which package managers are available
def check_manager(cmd):
    try:
        subprocess.run(cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True, timeout=5)
        return True
    except:
        return False

# Detect available managers once
def detect_managers():
    managers = {
        "brew": "brew --version",
        "apt": "apt --version",
        "winget": "winget --version",
        "scoop": "scoop --version",
        "choco": "choco --version",
        "snap": "snap version",
        "nix": "nix-env --version"
    }
    for name, cmd in managers.items():
        if check_manager(cmd):
            available_managers[name] = True

# Probe installability
def probe(cmd):
    try:
        subprocess.run(cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True, timeout=3)
        return True
    except:
        return False

def installability(name):
    name = name.lower()
    platforms = {}
    if "brew" in available_managers and probe(f"brew info {name}"):
        platforms["brew"] = f"brew install {name}"
    if "apt" in available_managers and probe(f"apt-cache show {name}"):
        platforms["apt"] = f"sudo apt install {name}"
    if "winget" in available_managers and probe(f"winget show {name}"):
        platforms["winget"] = f"winget install {name}"
    if "scoop" in available_managers and probe(f"scoop search {name}"):
        platforms["scoop"] = f"scoop install {name}"
    if "choco" in available_managers and probe(f"choco search {name}"):
        platforms["choco"] = f"choco install {name}"
    if "snap" in available_managers and probe(f"snap info {name}"):
        platforms["snap"] = f"sudo snap install {name}"
    if "nix" in available_managers and probe(f"nix-env -qaP {name}"):
        platforms["nix"] = f"nix-env -iA nixpkgs.{name}"
    return platforms

# Source 1: Awesome CLI Apps
def fetch_awesome_cli_apps():
    print("Fetching from awesome-cli-apps...")
    url = "https://raw.githubusercontent.com/agarrharr/awesome-cli-apps/master/readme.md"
    md = requests.get(url).text
    count = 0
    for line in md.splitlines():
        if line.startswith("- ["):
            match = re.match(r"- \[(.*?)\]\((.*?)\) - (.*)", line)
            if match:
                name, link, desc = match.groups()
                key = name.strip().lower()
                if key not in tools:
                    tools[key] = {
                        "name": name.strip(),
                        "description": desc.strip(),
                        "homepage": link.strip(),
                        "source": "awesome-cli-apps"
                    }
                    count += 1
    print(f"Found {count} tools from awesome-cli-apps")

# Source 2: toolleeo/cli-apps (currently unavailable - 404 error)
def fetch_toolleeo_cli_apps():
    print("Skipping toolleeo/cli-apps - repository structure has changed or moved")
    pass

# Source 3: LinuxLinks
def fetch_linuxlinks_cli_apps():
    print("Fetching from LinuxLinks...")
    url = "https://www.linuxlinks.com/100-great-must-have-cli-linux-applications/"
    html = requests.get(url).text
    soup = BeautifulSoup(html, "html.parser")
    count = 0
    for li in soup.select("div.entry-content li"):
        text = li.get_text()
        match = re.match(r"^(.*?) – (.*)", text)
        if match:
            name, desc = match.groups()
            key = name.strip().lower()
            if key not in tools:
                tools[key] = {
                    "name": name.strip(),
                    "description": desc.strip(),
                    "homepage": "",
                    "source": "linuxlinks"
                }
                count += 1
    print(f"Found {count} tools from LinuxLinks")

# Run all fetchers
detect_managers()
fetch_awesome_cli_apps()
fetch_toolleeo_cli_apps()
# fetch_linuxlinks_cli_apps()  # Disabled - different page structure

# Probe installability (limit to first 50 for testing)
import sys
limit = 50 if len(sys.argv) == 1 else int(sys.argv[1]) if len(sys.argv) > 1 and sys.argv[1].isdigit() else len(tools)
limited_tools = dict(list(tools.items())[:limit])

print(f"\nChecking installability for {len(limited_tools)} tools (limited from {len(tools)} total)...")
table_rows = []
for i, (key, tool) in enumerate(limited_tools.items(), 1):
    if i % 5 == 0 or i == len(limited_tools):
        print(f"Progress: {i}/{len(limited_tools)} tools checked...")
    installs = installability(tool["name"])
    tool["install"] = installs
    row = [tool["name"]] + [("✅" if mgr in installs else "") for mgr in available_managers]
    table_rows.append(row)

# Print table
headers = ["Tool"] + list(available_managers.keys())
print(tabulate(table_rows, headers=headers, tablefmt="grid"))

# Output to JSON (for reference)
with open("cli_tools_with_installs.json", "w") as f:
    json.dump(list(tools.values()), f, indent=2)

# Output to source-organized CCL files
print("\nGenerating source-organized CCL files...")

# Organize tools by package manager (now includes description)
tools_by_manager = {}
for mgr in available_managers:
    tools_by_manager[mgr] = []

for tool in tools.values():
    if "install" in tool and tool["install"]:
        for mgr in tool["install"].keys():
            if mgr in tools_by_manager:
                # Include both name and description
                tools_by_manager[mgr].append({
                    "name": tool["name"],
                    "description": tool.get("description", "")
                })

# Write CCL files for each manager using isolated CCL writing function
os.makedirs("generated_sources", exist_ok=True)

for mgr, tool_list in tools_by_manager.items():
    if not tool_list:
        continue

    ccl_path = f"generated_sources/{mgr}.ccl"
    write_ccl_file(ccl_path, mgr, tool_list)
    print(f"  ✅ Generated {ccl_path} ({len(tool_list)} packages)")

print(f"\nCollected {len(tools)} unique tools with installability metadata.")
print(f"Generated CCL files in generated_sources/ directory")
