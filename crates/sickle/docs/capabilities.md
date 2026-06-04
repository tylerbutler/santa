# Sickle CCL Capabilities

> **Auto-generated** from test data - DO NOT EDIT
> 
> Run `just sickle-capabilities` to regenerate

This documentation is extracted from the comprehensive test suite in `tests/test_data/*.json`.
The test files are the source of truth for all supported features and behaviors.

## Overview

- **Functions**: 14 distinct functions tested
- **Behaviors**: 11 distinct behaviors tested
- **Total test cases**: 379

## Functions

Core Model API methods and operations covered by the test suite:

| Function | Test Cases | Description |
|----------|------------|-------------|
| `build_hierarchy` | 83 | Validation type: `build_hierarchy` |
| `canonical_format` | 11 | Validation type: `canonical_format` |
| `compose_associative` | 3 | Validation type: `compose_associative` |
| `filter` | 3 | Validation type: `filter` |
| `get_bool` | 12 | Validation type: `get_bool` |
| `get_float` | 6 | Validation type: `get_float` |
| `get_int` | 11 | Validation type: `get_int` |
| `get_list` | 43 | Validation type: `get_list` |
| `get_string` | 11 | Validation type: `get_string` |
| `identity_left` | 3 | Validation type: `identity_left` |
| `identity_right` | 3 | Validation type: `identity_right` |
| `parse` | 166 | Validation type: `parse` |
| `parse_indented` | 12 | Validation type: `parse_indented` |
| `round_trip` | 12 | Validation type: `round_trip` |

### Function Examples

#### `build_hierarchy`

**Test coverage**: 83 test cases

**Example usage from tests**:

- **spacing_loose_multiline_various_build_hierarchy** (`api_whitespace_behaviors.json`)
- **tabs_preserve_in_value_build_hierarchy** (`api_whitespace_behaviors.json`)

#### `canonical_format`

**Test coverage**: 11 test cases

**Example usage from tests**:

- **spacing_canonical_format_normalizes_loose_canonical_format** (`api_whitespace_behaviors.json`)
- **tabs_canonical_format_preserve_canonical_format** (`api_whitespace_behaviors.json`)

#### `compose_associative`

**Test coverage**: 3 test cases

**Example usage from tests**:

- **semigroup_associativity_basic_compose_associative** (`property_algebraic.json`)
- **semigroup_associativity_nested_compose_associative** (`property_algebraic.json`)

#### `filter`

**Test coverage**: 3 test cases

**Example usage from tests**:

- **comment_extension_filter** (`api_comments.json`)
- **comment_syntax_slash_equals_filter** (`api_comments.json`)

#### `get_bool`

**Test coverage**: 12 test cases

**Example usage from tests**:

- **parse_boolean_true_get_bool** (`api_typed_access.json`)
- **parse_boolean_yes_get_bool** (`api_typed_access.json`)

#### `get_float`

**Test coverage**: 6 test cases

**Example usage from tests**:

- **parse_basic_float_get_float** (`api_typed_access.json`)
- **parse_zero_values_get_float** (`api_typed_access.json`)

#### `get_int`

**Test coverage**: 11 test cases

**Example usage from tests**:

- **parse_basic_integer_get_int** (`api_typed_access.json`)
- **parse_negative_integer_get_int** (`api_typed_access.json`)

#### `get_list`

**Test coverage**: 43 test cases

**Example usage from tests**:

- **basic_list_from_duplicates_get_list** (`api_list_access.json`)
- **large_list_get_list** (`api_list_access.json`)

#### `get_string`

**Test coverage**: 11 test cases

**Example usage from tests**:

- **tabs_preserve_in_value_get_string** (`api_whitespace_behaviors.json`)
- **tabs_preserve_leading_tab_get_string** (`api_whitespace_behaviors.json`)

#### `identity_left`

**Test coverage**: 3 test cases

**Example usage from tests**:

- **monoid_left_identity_basic_identity_left** (`property_algebraic.json`)
- **monoid_left_identity_nested_identity_left** (`property_algebraic.json`)

#### `identity_right`

**Test coverage**: 3 test cases

**Example usage from tests**:

- **monoid_right_identity_basic_identity_right** (`property_algebraic.json`)
- **monoid_right_identity_nested_identity_right** (`property_algebraic.json`)

#### `parse`

**Test coverage**: 166 test cases

**Example usage from tests**:

- **spacing_strict_standard_format_parse** (`api_whitespace_behaviors.json`)
- **spacing_loose_no_spaces_parse** (`api_whitespace_behaviors.json`)

#### `parse_indented`

**Test coverage**: 12 test cases

**Example usage from tests**:

- **multiline_section_header_value_parse_indented** (`api_proposed_behavior.json`)
- **unindented_multiline_becomes_continuation_parse_indented** (`api_proposed_behavior.json`)

#### `round_trip`

**Test coverage**: 12 test cases

**Example usage from tests**:

- **round_trip_property_basic_round_trip** (`property_algebraic.json`)
- **round_trip_property_nested_round_trip** (`property_algebraic.json`)

## Behaviors

Parser behaviors and configuration options tested by the suite:

| Behavior | Test Cases |
|----------|------------|
| `array_order_insertion` | 9 |
| `array_order_lexicographic` | 25 |
| `boolean_lenient` | 6 |
| `boolean_strict` | 7 |
| `crlf_preserve_literal` | 2 |
| `list_coercion_disabled` | 12 |
| `list_coercion_enabled` | 18 |
| `loose_spacing` | 11 |
| `strict_spacing` | 1 |
| `tabs_preserve` | 11 |
| `tabs_to_spaces` | 7 |

### Behavior Examples

#### `array_order_insertion`

**Test coverage**: 9 test cases

**Example usage from tests**:

- **list_with_comments_build_hierarchy** (`api_list_access.json`)
- **list_with_comments_get_list** (`api_list_access.json`)

#### `array_order_lexicographic`

**Test coverage**: 25 test cases

**Example usage from tests**:

- **list_with_comments_lexicographic_build_hierarchy** (`api_list_access.json`)
- **list_with_comments_lexicographic_get_list** (`api_list_access.json`)

#### `boolean_lenient`

**Test coverage**: 6 test cases

**Example usage from tests**:

- **parse_boolean_true_get_bool** (`api_typed_access.json`)
- **parse_boolean_yes_get_bool** (`api_typed_access.json`)

#### `boolean_strict`

**Test coverage**: 7 test cases

**Example usage from tests**:

- **parse_boolean_true_get_bool** (`api_typed_access.json`)
- **parse_boolean_yes_strict_literal_get_bool** (`api_typed_access.json`)

#### `crlf_preserve_literal`

**Test coverage**: 2 test cases

**Example usage from tests**:

- **canonical_format_line_endings_reference_behavior_parse** (`api_reference_compliant.json`)
- **canonical_format_line_endings_reference_behavior_canonical_format** (`api_reference_compliant.json`)

#### `list_coercion_disabled`

**Test coverage**: 12 test cases

**Example usage from tests**:

- **bare_list_error_not_a_list_get_list** (`api_list_access.json`)
- **single_item_as_list_reference_get_list** (`api_reference_compliant.json`)

#### `list_coercion_enabled`

**Test coverage**: 18 test cases

**Example usage from tests**:

- **basic_list_from_duplicates_get_list** (`api_list_access.json`)
- **large_list_get_list** (`api_list_access.json`)

#### `loose_spacing`

**Test coverage**: 11 test cases

**Example usage from tests**:

- **spacing_strict_standard_format_parse** (`api_whitespace_behaviors.json`)
- **spacing_loose_no_spaces_parse** (`api_whitespace_behaviors.json`)

#### `strict_spacing`

**Test coverage**: 1 test cases

**Example usage from tests**:

- **spacing_strict_standard_format_parse** (`api_whitespace_behaviors.json`)

#### `tabs_preserve`

**Test coverage**: 11 test cases

**Example usage from tests**:

- **tabs_preserve_in_value_parse** (`api_whitespace_behaviors.json`)
- **tabs_preserve_in_value_build_hierarchy** (`api_whitespace_behaviors.json`)

#### `tabs_to_spaces`

**Test coverage**: 7 test cases

**Example usage from tests**:

- **tabs_to_spaces_in_value_parse** (`api_whitespace_behaviors.json`)
- **tabs_to_spaces_in_value_build_hierarchy** (`api_whitespace_behaviors.json`)

## Running Tests

To run the comprehensive test suite:

```bash
just test-ccl
```

This runs all test cases from the JSON test data files and provides detailed
pass/fail statistics per function and behavior.
