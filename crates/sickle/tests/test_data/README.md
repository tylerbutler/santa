# CCL Test Data

This directory contains JSON test suites from the [ccl-test-data](https://github.com/tylerbutler/ccl-test-data) repository.

## Overview

The test files provide comprehensive test cases for CCL (Categorical Configuration Language) parsing and validation, following the specification at [ccl.tylerbutler.com](https://ccl.tylerbutler.com).

## Test Files

- **api_core_ccl_parsing.json** - Core parsing functionality tests (basic key-value pairs, multiline values, whitespace handling, etc.)
- **api_comments.json** - Comment handling tests
- **api_typed_access.json** - Typed accessor tests (get_string, get_int, get_bool, get_float, etc.)

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

## Current Status

Not all tests are expected to pass, as sickle is under active development. The test infrastructure allows individual test cases to fail while still reporting which tests pass, making it easy to track implementation progress.

## Updating Test Data

To add more test files:

1. Download JSON files from [ccl-test-data/generated_tests](https://github.com/tylerbutler/ccl-test-data/tree/main/generated_tests)
2. Place them in this directory
3. Add corresponding test functions in `integration_tests.rs` using `TestSuite::from_file()`

The test helpers will automatically discover and load all `.json` files in this directory.
