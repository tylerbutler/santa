# Sickle CCL Capabilities

> **Auto-generated** from test data - DO NOT EDIT
> 
> Run `just sickle-capabilities` to regenerate

This documentation is extracted from the comprehensive test suite in `tests/test_data/*.json`.
The test files are the source of truth for all supported features and behaviors.

## Overview

- **Functions**: 13 distinct functions tested
- **Behaviors**: 17 distinct behaviors tested
- **Total test cases**: 1565

## Functions

Core Model API methods and operations covered by the test suite:

| Function | Test Cases | Description |
|----------|------------|-------------|
| `build_hierarchy` | 195 | Validation type: `build_hierarchy` |
| `build_model` | 200 | Validation type: `build_model` |
| `canonical_format` | 16 | Validation type: `canonical_format` |
| `compose` | 9 | Validation type: `compose_associative` |
| `filter` | 14 | Validation type: `filter` |
| `get_bool` | 22 | Validation type: `get_bool` |
| `get_float` | 10 | Validation type: `get_float` |
| `get_int` | 16 | Validation type: `get_int` |
| `get_list` | 44 | Validation type: `get_list` |
| `get_string` | 82 | Validation type: `get_string` |
| `parse` | 483 | Validation type: `parse` |
| `parse_indented` | 405 | Validation type: `build_hierarchy` |
| `print` | 69 | Validation type: `print` |

### Function Examples

#### `build_hierarchy`

**Test coverage**: 195 test cases

**Example usage from tests**:

- **s99_fuzz_val_path_0_build_hierarchy** (`api_fuzz_special_chars_seed99.json`)
- **s99_fuzz_val_item_1_build_hierarchy** (`api_fuzz_special_chars_seed99.json`)

#### `build_model`

**Test coverage**: 200 test cases

**Example usage from tests**:

- **s99_fuzz_val_path_0_build_model** (`api_fuzz_special_chars_seed99.json`)
- **s99_fuzz_val_item_1_build_model** (`api_fuzz_special_chars_seed99.json`)

#### `canonical_format`

**Test coverage**: 16 test cases

**Example usage from tests**:

- **canonical_format_empty_values_ocaml_reference_canonical_format** (`api_reference_compliant.json`)
- **canonical_format_unicode_ocaml_reference_canonical_format** (`api_reference_compliant.json`)

#### `compose`

**Test coverage**: 9 test cases

**Example usage from tests**:

- **semigroup_associativity_basic_compose_associative** (`property_algebraic.json`)
- **semigroup_associativity_nested_compose_associative** (`property_algebraic.json`)

#### `filter`

**Test coverage**: 14 test cases

**Example usage from tests**:

- **filter_by_key_equality_filter** (`api_filter_predicates.json`)
- **filter_by_value_not_empty_filter** (`api_filter_predicates.json`)

#### `get_bool`

**Test coverage**: 22 test cases

**Example usage from tests**:

- **parse_boolean_true_get_bool** (`api_typed_access.json`)
- **parse_boolean_yes_get_bool** (`api_typed_access.json`)

#### `get_float`

**Test coverage**: 10 test cases

**Example usage from tests**:

- **parse_basic_float_get_float** (`api_typed_access.json`)
- **parse_zero_values_get_float** (`api_typed_access.json`)

#### `get_int`

**Test coverage**: 16 test cases

**Example usage from tests**:

- **parse_basic_integer_get_int** (`api_typed_access.json`)
- **parse_negative_integer_get_int** (`api_typed_access.json`)

#### `get_list`

**Test coverage**: 44 test cases

**Example usage from tests**:

- **single_item_as_list_reference_get_list** (`api_reference_compliant.json`)
- **mixed_duplicate_single_keys_reference_get_list** (`api_reference_compliant.json`)

#### `get_string`

**Test coverage**: 82 test cases

**Example usage from tests**:

- **s99_fuzz_val_path_0_get_string** (`api_fuzz_special_chars_seed99.json`)
- **s99_fuzz_val_item_1_get_string** (`api_fuzz_special_chars_seed99.json`)

#### `parse`

**Test coverage**: 483 test cases

**Example usage from tests**:

- **s99_fuzz_single_underscore_parse** (`api_fuzz_special_chars_seed99.json`)
- **s99_fuzz_single_backslash_parse** (`api_fuzz_special_chars_seed99.json`)

#### `parse_indented`

**Test coverage**: 405 test cases

**Example usage from tests**:

- **s99_fuzz_val_path_0_build_hierarchy** (`api_fuzz_special_chars_seed99.json`)
- **s99_fuzz_val_path_0_build_model** (`api_fuzz_special_chars_seed99.json`)

#### `print`

**Test coverage**: 69 test cases

**Example usage from tests**:

- **round_trip_basic_print** (`property_round_trip.json`)
- **round_trip_basic_round_trip** (`property_round_trip.json`)

## Behaviors

Parser behaviors and configuration options tested by the suite:

| Behavior | Test Cases |
|----------|------------|
| `array_order_insertion` | 30 |
| `array_order_lexicographic` | 25 |
| `boolean_lenient` | 10 |
| `boolean_strict` | 16 |
| `continuation_tab_to_space` | 4 |
| `crlf_normalize_to_lf` | 17 |
| `crlf_preserve_literal` | 17 |
| `delimiter_first_equals` | 9 |
| `delimiter_prefer_spaced` | 12 |
| `indent_spaces` | 13 |
| `indent_tabs` | 8 |
| `list_coercion_disabled` | 12 |
| `list_coercion_enabled` | 17 |
| `multiline_values` | 23 |
| `path_traversal` | 30 |
| `toplevel_indent_preserve` | 2 |
| `toplevel_indent_strip` | 2 |

### Behavior Examples

#### `array_order_insertion`

**Test coverage**: 30 test cases

**Example usage from tests**:

- **list_with_comments_build_hierarchy** (`api_list_access.json`)
- **list_with_comments_get_list** (`api_list_access.json`)

#### `array_order_lexicographic`

**Test coverage**: 25 test cases

**Example usage from tests**:

- **mixed_duplicate_single_keys_reference_build_hierarchy** (`api_reference_compliant.json`)
- **mixed_duplicate_single_keys_reference_get_list** (`api_reference_compliant.json`)

#### `boolean_lenient`

**Test coverage**: 10 test cases

**Example usage from tests**:

- **parse_boolean_true_get_bool** (`api_typed_access.json`)
- **parse_boolean_yes_get_bool** (`api_typed_access.json`)

#### `boolean_strict`

**Test coverage**: 16 test cases

**Example usage from tests**:

- **parse_boolean_true_get_bool** (`api_typed_access.json`)
- **parse_boolean_yes_strict_literal_get_bool** (`api_typed_access.json`)

#### `continuation_tab_to_space`

**Test coverage**: 4 test cases

**Example usage from tests**:

- **tabs_as_whitespace_multiline_parse** (`api_whitespace_behaviors.json`)
- **tabs_as_whitespace_mixed_indent_parse** (`api_whitespace_behaviors.json`)

#### `crlf_normalize_to_lf`

**Test coverage**: 17 test cases

**Example usage from tests**:

- **crlf_normalize_to_lf_basic_parse** (`api_whitespace_behaviors.json`)
- **crlf_normalize_to_lf_basic_build_hierarchy** (`api_whitespace_behaviors.json`)

#### `crlf_preserve_literal`

**Test coverage**: 17 test cases

**Example usage from tests**:

- **canonical_format_line_endings_reference_behavior_parse** (`api_reference_compliant.json`)
- **canonical_format_line_endings_reference_behavior_canonical_format** (`api_reference_compliant.json`)

#### `delimiter_first_equals`

**Test coverage**: 9 test cases

**Example usage from tests**:

- **delimiter_first_url_with_query_params_parse** (`api_edge_cases.json`)
- **delimiter_first_url_with_query_params_build_hierarchy** (`api_edge_cases.json`)

#### `delimiter_prefer_spaced`

**Test coverage**: 12 test cases

**Example usage from tests**:

- **url_with_query_params_as_key_parse** (`api_edge_cases.json`)
- **url_with_query_params_as_key_build_hierarchy** (`api_edge_cases.json`)

#### `indent_spaces`

**Test coverage**: 13 test cases

**Example usage from tests**:

- **canonical_format_unicode_ocaml_reference_canonical_format** (`api_reference_compliant.json`)
- **canonical_format_line_endings_reference_behavior_canonical_format** (`api_reference_compliant.json`)

#### `indent_tabs`

**Test coverage**: 8 test cases

**Example usage from tests**:

- **print_nested_indent_tabs_print** (`api_whitespace_behaviors.json`)
- **print_deeply_nested_indent_tabs_print** (`api_whitespace_behaviors.json`)

#### `list_coercion_disabled`

**Test coverage**: 12 test cases

**Example usage from tests**:

- **single_item_as_list_reference_get_list** (`api_reference_compliant.json`)
- **mixed_duplicate_single_keys_reference_get_list** (`api_reference_compliant.json`)

#### `list_coercion_enabled`

**Test coverage**: 17 test cases

**Example usage from tests**:

- **basic_list_from_duplicates_get_list** (`api_list_access.json`)
- **large_list_get_list** (`api_list_access.json`)

#### `multiline_values`

**Test coverage**: 23 test cases

**Example usage from tests**:

- **round_trip_multiline_values_parse** (`property_round_trip.json`)
- **round_trip_multiline_values_round_trip** (`property_round_trip.json`)

#### `path_traversal`

**Test coverage**: 30 test cases

**Example usage from tests**:

- **nested_list_access_reference_get_list** (`api_reference_compliant.json`)
- **complex_mixed_list_scenarios_reference_get_list** (`api_reference_compliant.json`)

#### `toplevel_indent_preserve`

**Test coverage**: 2 test cases

**Example usage from tests**:

- **toplevel_indent_preserve_canonical_canonical_format** (`api_whitespace_behaviors.json`)
- **toplevel_indent_preserve_round_trip_round_trip** (`api_whitespace_behaviors.json`)

#### `toplevel_indent_strip`

**Test coverage**: 2 test cases

**Example usage from tests**:

- **toplevel_indent_strip_canonical_canonical_format** (`api_whitespace_behaviors.json`)
- **toplevel_indent_strip_round_trip_round_trip** (`api_whitespace_behaviors.json`)

## Running Tests

To run the comprehensive test suite:

```bash
just test-ccl
```

This runs all test cases from the JSON test data files and provides detailed
pass/fail statistics per function and behavior.
