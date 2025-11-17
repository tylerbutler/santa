# CCL Test Data

This directory contains JSON test suites from the [ccl-test-data](https://github.com/tylerbutler/ccl-test-data) repository.

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
# Run comprehensive test covering all 327 test cases from all JSON files
just test-ccl

# Or run directly with cargo
cargo test -p sickle test_all_ccl_suites_comprehensive -- --nocapture
```

This will run all test cases from all 13 JSON files and show per-suite results.

## Current Status

**110 out of 327 tests passing (33.6%)** as of the latest run:

- ✅ **api_core_ccl_parsing**: 8/8 passing (100%)
- ✅ **api_errors**: 6/6 passing (100%)
- ✅ **api_edge_cases**: 21/35 passing (60%)
- ⚠️ **api_typed_access**: 25/74 passing (34%)
- ⚠️ **api_comments**: 0/6 passing (0%) - feature not implemented
- And more...

Not all tests are expected to pass as sickle is under active development. The test infrastructure gracefully handles failures and shows which specific features are implemented, making it easy to track implementation progress.

## Updating Test Data

To download/update all test files from the ccl-test-data repository:

```bash
just download-ccl-tests
```

This task clones the [ccl-test-data repository](https://github.com/tylerbutler/ccl-test-data) to a temporary location and copies all JSON files from the `generated_tests/` directory. This ensures you always have the complete set of test files.

The test helpers automatically discover and load all `.json` files in this directory when running tests.
