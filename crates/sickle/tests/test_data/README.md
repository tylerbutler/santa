# CCL Test Data

This directory contains JSON test suites from the [ccl-test-data](https://github.com/CatConfLang/ccl-test-data) repository.

## Downloading Test Data

Test data JSON files are **not committed to git** — they are downloaded from ccl-test-data GitHub releases. To download or update test data:

```bash
# Download pinned version (skips if already current)
just download-ccl-tests

# Download the latest release
just download-ccl-tests latest

# Download a specific version
just download-ccl-tests v0.6.2

# Force re-download even if already current
just download-ccl-tests latest true
```

A `.version` file tracks the currently downloaded release to avoid unnecessary re-downloads.

In CI, the `.github/actions/download-ccl-tests` composite action handles this automatically before test runs.

## Overview

The test files provide comprehensive test cases for CCL (Categorical Configuration Language) parsing and validation, following the specification at [ccl.tylerbutler.com](https://ccl.tylerbutler.com).

## Test Files

This directory contains all 13 test suites from the ccl-test-data repository:

### Core API Tests
- **api_core_ccl_parsing.json** - Core parsing functionality tests
- **api_core_ccl_hierarchy.json** - Hierarchy building tests
- **api_core_ccl_integration.json** - Integration tests
- **api_comments.json** - Comment handling tests
- **api_typed_access.json** - Typed accessor tests (get_string, get_int, get_bool, get_float, etc.)
- **api_advanced_processing.json** - Advanced processing operations
- **api_list_access.json** - List access tests

### Edge Cases & Compliance
- **api_edge_cases.json** - Edge case handling
- **api_errors.json** - Error handling tests
- **api_proposed_behavior.json** - Proposed CCL behavior tests
- **api_reference_compliant.json** - Reference implementation compliance tests

### Property-Based Tests
- **property_algebraic.json** - Algebraic property tests
- **property_round_trip.json** - Round-trip property tests

## JSON Structure

Each test file contains a `tests` array with test cases in this format:

```json
{
  "name": "test_name",
  "input": "CCL input string",
  "validation": "parse|get_string|get_int|...",
  "expected": {
    "count": 2,
    "entries": [
      {"key": "name", "value": "value"}
    ]
  },
  "features": ["comments", "multiline", "unicode"],
  "behaviors": [],
  "variants": []
}
```

## Usage

The test infrastructure in `test_helpers.rs` loads these JSON files and runs them against the sickle parser. Tests are organized by validation type:

- `parse` - Tests basic parsing functionality
- `get_string`, `get_int`, etc. - Tests typed access methods
- `build_hierarchy`, `filter`, `combine` - Tests advanced operations

### Running Tests

```bash
# Run comprehensive test covering all test cases from all JSON files
just test-ccl

# Or run directly with cargo
cargo test -p sickle test_all_ccl_suites_comprehensive -- --nocapture
```

This will run all test cases from all JSON files and show per-suite results.

The test helpers automatically discover and load all `.json` files in this directory when running tests.
