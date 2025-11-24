# Sickle CCL Capabilities

> **Auto-generated** from test data - DO NOT EDIT
> 
> Run `just sickle-capabilities` to regenerate

This documentation is extracted from the comprehensive test suite in `tests/test_data/*.json`.
The test files are the source of truth for all supported features and behaviors.

## Overview

- **Functions**: 11 distinct functions tested
- **Behaviors**: 8 distinct behaviors tested
- **Total test cases**: 327

## Functions

Core Model API methods and operations covered by the test suite:

| Function | Test Cases | Description |
|----------|------------|-------------|
| `build_hierarchy` | 67 | Validation type: `build_hierarchy` |
| `canonical_format` | 14 | Validation type: `canonical_format` |
| `filter` | 3 | Validation type: `filter` |
| `get_bool` | 12 | Validation type: `get_bool` |
| `get_float` | 6 | Validation type: `get_float` |
| `get_int` | 11 | Validation type: `get_int` |
| `get_list` | 32 | Validation type: `get_list` |
| `get_string` | 7 | Validation type: `get_string` |
| `parse` | 153 | Validation type: `parse` |
| `parse_indented` | 10 | Validation type: `parse_indented` |
| `round_trip` | 12 | Validation type: `round_trip` |

### Function Examples

#### `build_hierarchy`

**Test coverage**: 67 test cases

**Example usage from tests**:

- **single_item_as_list_reference_build_hierarchy** (`api_reference_compliant.json`)
  ```ccl
  item = single
  ```
- **mixed_duplicate_single_keys_reference_build_hierarchy** (`api_reference_compliant.json`)
  ```ccl
  ports = 80
ports = 443
host = localhost
  ```

#### `canonical_format`

**Test coverage**: 14 test cases

**Example usage from tests**:

- **canonical_format_empty_values_ocaml_reference_canonical_format** (`api_reference_compliant.json`)
  ```ccl
  empty_key =
  ```
- **canonical_format_tab_preservation_ocaml_reference_canonical_format** (`api_reference_compliant.json`)
  ```ccl
  value_with_tabs = text		with	tabs	
  ```

#### `filter`

**Test coverage**: 3 test cases

**Example usage from tests**:

- **comment_extension_filter** (`api_comments.json`)
  ```ccl
  /= This is an environment section
port = 8080
serve = index.html
/= Database section
mode = in-memor...
  ```
- **comment_syntax_slash_equals_filter** (`api_comments.json`)
  ```ccl
  /= this is a comment
  ```

#### `get_bool`

**Test coverage**: 12 test cases

**Example usage from tests**:

- **parse_boolean_true_get_bool** (`api_typed_access.json`)
  ```ccl
  enabled = true
  ```
- **parse_boolean_yes_get_bool** (`api_typed_access.json`)
  ```ccl
  active = yes
  ```

#### `get_float`

**Test coverage**: 6 test cases

**Example usage from tests**:

- **parse_basic_float_get_float** (`api_typed_access.json`)
  ```ccl
  temperature = 98.6
  ```
- **parse_zero_values_get_float** (`api_typed_access.json`)
  ```ccl
  count = 0
distance = 0.0
disabled = no
  ```

#### `get_int`

**Test coverage**: 11 test cases

**Example usage from tests**:

- **parse_basic_integer_get_int** (`api_typed_access.json`)
  ```ccl
  port = 8080
  ```
- **parse_negative_integer_get_int** (`api_typed_access.json`)
  ```ccl
  offset = -42
  ```

#### `get_list`

**Test coverage**: 32 test cases

**Example usage from tests**:

- **single_item_as_list_reference_get_list** (`api_reference_compliant.json`)
  ```ccl
  item = single
  ```
- **mixed_duplicate_single_keys_reference_get_list** (`api_reference_compliant.json`)
  ```ccl
  ports = 80
ports = 443
host = localhost
  ```

#### `get_string`

**Test coverage**: 7 test cases

**Example usage from tests**:

- **parse_string_fallback_get_string** (`api_typed_access.json`)
  ```ccl
  name = Alice
  ```
- **parse_mixed_types_get_string** (`api_typed_access.json`)
  ```ccl
  host = localhost
port = 8080
ssl = true
timeout = 30.5
debug = off
  ```

#### `parse`

**Test coverage**: 153 test cases

**Example usage from tests**:

- **round_trip_whitespace_normalization_parse** (`property_round_trip.json`)
  ```ccl
    key  =  value  
  nested  = 
    sub  =  val  
  ```
- **round_trip_empty_keys_lists_parse** (`property_round_trip.json`)
  ```ccl
  = item1
= item2
regular = value
  ```

#### `parse_indented`

**Test coverage**: 10 test cases

**Example usage from tests**:

- **multiline_section_header_value_parse_indented** (`api_proposed_behavior.json`)
  ```ccl
    key = val
  ```
- **unindented_multiline_becomes_continuation_parse_indented** (`api_proposed_behavior.json`)
  ```ccl
    = val
  ```

#### `round_trip`

**Test coverage**: 12 test cases

**Example usage from tests**:

- **round_trip_basic_round_trip** (`property_round_trip.json`)
  ```ccl
  key = value
nested =
  sub = val
  ```
- **round_trip_whitespace_normalization_round_trip** (`property_round_trip.json`)
  ```ccl
    key  =  value  
  nested  = 
    sub  =  val  
  ```

## Behaviors

Parser behaviors and configuration options tested by the suite:

| Behavior | Test Cases |
|----------|------------|
| `boolean_lenient` | 24 |
| `boolean_strict` | 27 |
| `crlf_normalize_to_lf` | 4 |
| `crlf_preserve_literal` | 4 |
| `list_coercion_disabled` | 14 |
| `list_coercion_enabled` | 15 |
| `strict_spacing` | 3 |
| `tabs_preserve` | 7 |

### Behavior Examples

#### `boolean_lenient`

**Test coverage**: 24 test cases

**Example usage from tests**:

- **parse_boolean_true_parse** (`api_typed_access.json`)
  ```ccl
  enabled = true
  ```
- **parse_boolean_true_build_hierarchy** (`api_typed_access.json`)
  ```ccl
  enabled = true
  ```

#### `boolean_strict`

**Test coverage**: 27 test cases

**Example usage from tests**:

- **parse_boolean_true_parse** (`api_typed_access.json`)
  ```ccl
  enabled = true
  ```
- **parse_boolean_true_build_hierarchy** (`api_typed_access.json`)
  ```ccl
  enabled = true
  ```

#### `crlf_normalize_to_lf`

**Test coverage**: 4 test cases

**Example usage from tests**:

- **crlf_normalize_to_lf_proposed_parse** (`api_proposed_behavior.json`)
  ```ccl
  key1 = value1
key2 = value2

  ```
- **crlf_normalize_to_lf_proposed_canonical_format** (`api_proposed_behavior.json`)
  ```ccl
  key1 = value1
key2 = value2

  ```

#### `crlf_preserve_literal`

**Test coverage**: 4 test cases

**Example usage from tests**:

- **canonical_format_line_endings_reference_behavior_parse** (`api_reference_compliant.json`)
  ```ccl
  key1 = value1
key2 = value2

  ```
- **canonical_format_line_endings_reference_behavior_canonical_format** (`api_reference_compliant.json`)
  ```ccl
  key1 = value1
key2 = value2

  ```

#### `list_coercion_disabled`

**Test coverage**: 14 test cases

**Example usage from tests**:

- **single_item_as_list_reference_parse** (`api_reference_compliant.json`)
  ```ccl
  item = single
  ```
- **single_item_as_list_reference_build_hierarchy** (`api_reference_compliant.json`)
  ```ccl
  item = single
  ```

#### `list_coercion_enabled`

**Test coverage**: 15 test cases

**Example usage from tests**:

- **single_item_as_list_parse** (`api_proposed_behavior.json`)
  ```ccl
  item = single
  ```
- **single_item_as_list_build_hierarchy** (`api_proposed_behavior.json`)
  ```ccl
  item = single
  ```

#### `strict_spacing`

**Test coverage**: 3 test cases

**Example usage from tests**:

- **canonical_format_consistent_spacing_ocaml_reference_canonical_format** (`api_reference_compliant.json`)
  ```ccl
  key1=value1
key2  =  value2
key3	=	value3
  ```
- **canonical_format_consistent_spacing_parse** (`api_proposed_behavior.json`)
  ```ccl
  key1=value1
key2  =  value2
key3	=	value3
  ```

#### `tabs_preserve`

**Test coverage**: 7 test cases

**Example usage from tests**:

- **canonical_format_tab_preservation_ocaml_reference_canonical_format** (`api_reference_compliant.json`)
  ```ccl
  value_with_tabs = text		with	tabs	
  ```
- **key_with_tabs_parse** (`api_edge_cases.json`)
  ```ccl
  	key	=	value
  ```

## Running Tests

To run the comprehensive test suite:

```bash
just test-ccl
```

This runs all test cases from the JSON test data files and provides detailed
pass/fail statistics per function and behavior.
