# CCL Test Failures Analysis

**Status**: 16 test failures identified and categorized (as of 2025-11-19)
**Test Suite**: Comprehensive CCL test suite with 327 total tests
**Passing**: 215 tests (66%)
**Context**: Post design-principles refactoring

## Failure Categories

### 1. List Type Semantics (10 failures) - HIGH COMPLEXITY

**Issue**: `get_list()` returns values for numeric/boolean literals when it should return empty.

**Failing Tests**:
- `list_with_numbers_reference_get_list`
- `list_with_booleans_reference_get_list`
- `list_with_whitespace_reference_get_list`
- `deeply_nested_list_reference_get_list`
- `list_with_unicode_reference_get_list`
- `list_with_special_characters_reference_get_list`
- `complex_mixed_list_scenarios_reference_get_list`
- `list_with_whitespace_reference_build_hierarchy`
- `list_with_numbers_reference_get_list` (duplicate listing)

**Example**:
```
Input: "numbers = 1\nnumbers = 42\nnumbers = -17\nnumbers = 0"
Expected: get_list("numbers") → [] (empty)
Actual: get_list("numbers") → ["1", "42", "-17", "0"]
```

**Root Cause**: `get_list()` in `src/model.rs:175-187` returns all keys when `len() >= 2`, but should filter out values that are parseable as numbers or booleans.

**Fix Strategy**:
```rust
pub(crate) fn as_list(&self) -> Vec<String> {
    if self.len() >= 2 {
        self.keys()
            .filter(|k| {
                // Only include if NOT parseable as number or boolean
                k.parse::<i64>().is_err() &&
                k.parse::<f64>().is_err() &&
                !is_boolean_literal(k)
            })
            .cloned()
            .collect()
    } else {
        Vec::new()
    }
}

fn is_boolean_literal(s: &str) -> bool {
    matches!(s, "true" | "false" | "yes" | "no")
}
```

**Complexity**: Medium - Requires understanding CCL type semantics and how typed accessors (`get_int`, `get_bool`, `get_list`) should interact.

### 2. List Ordering (2 failures) - MEDIUM COMPLEXITY

**Issue**: List items appear in wrong order.

**Failing Tests**:
- `mixed_duplicate_single_keys_reference_build_hierarchy`
  - Expected: ports[0] = "443"
  - Actual: ports[0] = "80"
- `complex_mixed_list_scenarios_reference_build_hierarchy`
  - Expected: hosts[0] = "backup"
  - Actual: hosts[0] = "primary"

**Root Cause**: Unclear - could be:
1. Insertion order vs. expected order mismatch
2. Last-wins semantics for duplicate keys
3. Test data expectations don't match implementation

**Investigation Needed**:
- Check CCL specification for duplicate key handling
- Examine OCaml reference implementation behavior
- Verify IndexMap insertion order semantics

### 3. Whitespace/Tab Handling (4 failures) - MEDIUM COMPLEXITY

**Issue**: Parser doesn't preserve/normalize whitespace correctly.

**Failing Tests**:
- `key_with_tabs_parse`
  - Input: `"\tkey\t=\tvalue"`
  - Expected: `key="key", value="\tvalue"`
  - Behavior: `tabs_preserve`

- `spaces_vs_tabs_continuation_ocaml_reference_parse_indented`
  - Mixed space/tab continuation handling
  - Behavior: `tabs_preserve`

- `key_with_newline_before_equals_parse`
  - Expected: `key="key", value="val"`
  - Newline before `=` sign

- `complex_multi_newline_whitespace_parse`
  - Complex multi-line whitespace normalization

**Root Cause**: Parser in `src/parser.rs` may not correctly:
- Trim leading whitespace from keys
- Preserve tabs in values when `tabs_preserve` behavior is set
- Handle newlines in specific positions

**Investigation Needed**:
- Review parser whitespace handling logic
- Check tab preservation vs. normalization behavior
- Verify against reference implementation test data

### 4. Line Ending Handling (1 failure) - LOW COMPLEXITY

**Failing Test**:
- `canonical_format_line_endings_reference_behavior_parse`
  - Expected: `key1=value1` entry not found
  - Behavior: `crlf_preserve_literal`

**Root Cause**: Likely CRLF vs LF handling in parser or canonical format generation.

### 5. Whitespace Normalization (1 failure) - LOW COMPLEXITY

**Failing Test**:
- `round_trip_whitespace_normalization_parse`
  - Expected: 1 entry
  - Actual: 2 entries
  - Property-based round-trip test

**Root Cause**: Whitespace normalization creates duplicate entries or doesn't normalize consistently.

## Test Success Metrics

```
Total Tests: 327
✅ Passing: 215 (66%)
❌ Failing: 16 (5%)
⊘ Skipped: 96 (29%)

By Suite:
✅ api_advanced_processing: 17/17 (100%)
✅ api_comments: 6/6 (100%)
✅ api_core_ccl_hierarchy: 12/12 (100%)
✅ api_core_ccl_integration: 12/12 (100%)
✅ api_core_ccl_parsing: 8/8 (100%)
⚠️  api_edge_cases: 31/35 (89%)
✅ api_errors: 6/6 (100%)
✅ api_list_access: 21/21 (100%)
⊘  api_proposed_behavior: 0/60 (skipped - incompatible variant)
⚠️  api_reference_compliant: 27/44 (61% - 6 skipped, 11 failed)
⚠️  api_typed_access: 56/74 (76% - 18 skipped)
⚠️  property_algebraic: 12/15 (80% - 3 skipped)
⚠️  property_round_trip: 7/17 (41% - 9 skipped, 1 failed)
```

## Recommended Fix Order

### Phase 1: Quick Wins (Low Complexity)
1. Line ending handling (1 test)
2. Round-trip whitespace (1 test)

### Phase 2: Medium Complexity
3. Tab/whitespace preservation (4 tests)
4. List ordering semantics (2 tests)

### Phase 3: Complex Semantics
5. List type filtering (10 tests)
   - Requires deep understanding of CCL type system
   - May need specification clarification

## Resources

- **Test Data**: `/Volumes/Code/santa/crates/sickle/tests/test_data/*.json`
- **Main Test File**: `/Volumes/Code/santa/crates/sickle/tests/data_driven_tests.rs`
- **Model Implementation**: `/Volumes/Code/santa/crates/sickle/src/model.rs`
- **Parser**: `/Volumes/Code/santa/crates/sickle/src/parser.rs`

## References

- CCL Test Data Repository: https://github.com/tylerbutler/ccl-test-data
- Test Runner Implementation Guide: https://github.com/tylerbutler/ccl-test-data/blob/main/docs/test-runner-implementation-guide.md
- Test Runner Design Principles: https://github.com/tylerbutler/ccl-test-data/blob/main/docs/test-runner-design-principles.md
