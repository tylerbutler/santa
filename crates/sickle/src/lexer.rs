//! CCL lexing.
//!
//! This module owns the line-level rules of CCL: how a key is separated from
//! its value, how continuation lines attach to the entry above them, how
//! multi-line keys are folded, and how tabs and CRLF are treated. It produces
//! [`FlatEntry`] values annotated with the source lines they came from; the
//! syntax tree in [`crate::parser`] is built directly from those annotations,
//! so the lexer is the only place these rules live.

use crate::options::Options;

/// A logical source line.
///
/// Most logical lines map 1:1 to a physical line. A multi-line key — a run of
/// lines with no `=` followed by a line that has one — folds into a single
/// logical line spanning `start..=end`.
#[derive(Debug, Clone)]
pub(crate) struct LogicalLine {
    /// The (possibly folded) line text.
    pub(crate) text: String,
    /// First physical line index covered by this logical line.
    pub(crate) start: usize,
    /// Last physical line index covered by this logical line.
    pub(crate) end: usize,
    /// Whether several physical lines were folded together.
    pub(crate) folded: bool,
}

/// A lexed `key = value` pair plus the source lines it occupies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LexedEntry {
    /// The key text (empty for bare list items).
    pub(crate) key: String,
    /// The value text, exactly as CCL semantics define it.
    pub(crate) value: String,
    /// Indentation of the key line.
    pub(crate) indent: usize,
    /// Index of the logical line holding the key.
    pub(crate) header: usize,
    /// First physical line of the entry.
    pub(crate) start: usize,
    /// Last physical line of the entry's header (differs from `start` only for
    /// folded multi-line keys).
    pub(crate) header_end: usize,
    /// Last physical line that contributed content to the entry.
    pub(crate) end: usize,
    /// Byte offset of the `=` within the header's last physical line, when the
    /// entry actually had a delimiter.
    pub(crate) delimiter: Option<usize>,
}

/// Split `input` into physical lines, reporting whether it ended with a newline.
pub(crate) fn physical_lines(input: &str) -> (Vec<&str>, bool) {
    if input.is_empty() {
        return (Vec::new(), false);
    }
    let trailing_newline = input.ends_with('\n');
    let body = if trailing_newline {
        &input[..input.len() - 1]
    } else {
        input
    };
    (body.split('\n').collect(), trailing_newline)
}

/// Number of leading whitespace bytes on a line.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Trim whitespace, optionally keeping a trailing carriage return.
fn trim_line(s: &str, preserve_cr: bool) -> &str {
    if preserve_cr {
        s.trim_matches([' ', '\t'])
    } else {
        s.trim()
    }
}

/// Trim leading whitespace, optionally keeping a trailing carriage return.
fn trim_line_start(s: &str, preserve_cr: bool) -> &str {
    if preserve_cr {
        s.trim_start_matches([' ', '\t'])
    } else {
        s.trim_start()
    }
}

/// Locate the `=` that separates key from value.
///
/// * Strict spacing requires ` = `, or ` =` at end of line for an empty value.
/// * [`DelimiterStrategy::PreferSpaced`](crate::DelimiterStrategy::PreferSpaced)
///   tries ` = ` first so keys may contain a bare `=`.
/// * Otherwise the first `=` wins.
pub(crate) fn find_delimiter(s: &str, options: &Options) -> Option<usize> {
    if options.is_strict_spacing() {
        if let Some(pos) = s.find(" = ") {
            return Some(pos + 1);
        }
        if s.ends_with(" =") {
            return Some(s.len() - 1);
        }
        None
    } else if options.prefers_spaced_delimiter() {
        if let Some(pos) = s.find(" = ") {
            return Some(pos + 1);
        }
        if s.ends_with(" =") {
            return Some(s.len() - 1);
        }
        s.find('=')
    } else {
        s.find('=')
    }
}

/// Trim the whitespace between `=` and the start of an inline value.
///
/// With tabs preserved only spaces are removed, because a tab is then value
/// content rather than delimiter padding.
fn trim_inline_value<'a>(s: &'a str, options: &Options) -> &'a str {
    if options.preserves_tabs() {
        s.trim_start_matches(' ')
    } else {
        s.trim_start()
    }
}

/// Finish a value: strip trailing whitespace and apply tab handling.
fn finalize_value(value: &str, options: &Options) -> String {
    let trimmed = if options.preserves_crlf() {
        value.trim_end_matches([' ', '\t', '\n'])
    } else {
        value.trim_end()
    };
    options.process_tabs(trimmed).into_owned()
}

/// Fold multi-line keys into logical lines.
///
/// A line without `=` at or below the base indentation, not directly after a
/// complete entry, is a key continuation: it is joined with following lines up
/// to and including the next line containing `=`. Indented lines are value
/// continuations and pass through untouched.
pub(crate) fn logical_lines(lines: &[&str]) -> Vec<LogicalLine> {
    let mut result: Vec<LogicalLine> = Vec::new();
    let mut i = 0;
    let mut base_indent: Option<usize> = None;
    let mut previous_was_complete = false;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        let line_indent = indent_of(line);

        if trimmed.is_empty() && result.is_empty() {
            i += 1;
            continue;
        }

        if base_indent.is_none() && !trimmed.is_empty() {
            base_indent = Some(line_indent);
        }
        let base = base_indent.unwrap_or(0);
        let is_complete_entry = trimmed.contains('=');

        if line_indent > base {
            result.push(LogicalLine {
                text: line.to_string(),
                start: i,
                end: i,
                folded: false,
            });
            i += 1;
            continue;
        }

        if !trimmed.is_empty() && !trimmed.contains('=') && !previous_was_complete {
            let mut j = i + 1;
            let mut found_equals = false;
            while j < lines.len() {
                let next = lines[j];
                let next_trimmed = next.trim();
                if next_trimmed.is_empty() {
                    j += 1;
                    continue;
                }
                if indent_of(next) > line_indent {
                    break;
                }
                if next_trimmed.contains('=') {
                    found_equals = true;
                    break;
                }
                j += 1;
            }

            if found_equals && j < lines.len() {
                let mut key_parts = vec![trimmed];
                for part in lines.iter().take(j).skip(i + 1) {
                    let part_trimmed = part.trim();
                    if !part_trimmed.is_empty() && indent_of(part) <= line_indent {
                        key_parts.push(part_trimmed);
                    }
                }
                let mut text = key_parts.join(" ");
                text.push_str(lines[j].trim());
                result.push(LogicalLine {
                    text,
                    start: i,
                    end: j,
                    folded: true,
                });
                i = j + 1;
                previous_was_complete = true;
                continue;
            }
        }

        result.push(LogicalLine {
            text: line.to_string(),
            start: i,
            end: i,
            folded: false,
        });
        previous_was_complete = is_complete_entry;
        i += 1;
    }

    result
}

/// Lex a block of physical lines into entries.
///
/// Lines at the block's base indentation begin entries; more deeply indented
/// lines continue the entry above them.
pub(crate) fn lex(lines: &[&str], options: &Options) -> Vec<LexedEntry> {
    let logical = logical_lines(lines);
    let preserve_cr = options.preserves_crlf();

    let mut entries: Vec<LexedEntry> = Vec::new();
    let mut current: Option<PendingEntry> = None;
    let mut base_indent: Option<usize> = None;

    for (index, logical_line) in logical.iter().enumerate() {
        let line = logical_line.text.as_str();
        let indent = line.len() - trim_line_start(line, preserve_cr).len();
        let trimmed = trim_line(line, preserve_cr);

        if trimmed.is_empty() {
            if let Some(pending) = current.as_mut() {
                pending.value_lines.push(String::new());
            }
            continue;
        }

        if base_indent.is_none() {
            base_indent = Some(indent);
        }
        let base = base_indent.unwrap_or(0);

        if indent <= base && trimmed.contains('=') {
            if let Some(pending) = current.take() {
                entries.push(pending.finish(options));
            }

            let delimiter = find_delimiter(trimmed, options);
            match delimiter {
                Some(eq_pos) => {
                    let key = trimmed[..eq_pos].trim().to_string();
                    let inline = trim_inline_value(&trimmed[eq_pos + 1..], options).to_string();
                    let mut pending = PendingEntry::new(key, indent, index, logical_line);
                    pending.delimiter = Some(delimiter_offset_in_physical(
                        logical_line,
                        lines,
                        indent,
                        eq_pos,
                    ));
                    pending.value_lines.push(inline);
                    current = Some(pending);
                }
                None => {
                    // No valid delimiter (e.g. `key=value` under strict spacing):
                    // the whole line becomes a key with an empty value.
                    let mut pending =
                        PendingEntry::new(trimmed.to_string(), indent, index, logical_line);
                    pending.value_lines.push(String::new());
                    current = Some(pending);
                }
            }
        } else if let Some(pending) = current.as_mut() {
            if indent > pending.indent {
                pending.value_lines.push(line.to_string());
                pending.end = logical_line.end;
            } else {
                // A line with no delimiter at or below the current key's
                // indentation closes that entry and opens a new one. Its value
                // starts empty, so a following indented line becomes the value
                // verbatim — including its indentation, which is what marks it
                // as a nested key rather than a multi-line scalar.
                let finished = current.take().expect("checked above");
                entries.push(finished.finish(options));
                current = Some(PendingEntry::new(
                    trimmed.to_string(),
                    indent,
                    index,
                    logical_line,
                ));
            }
        }
    }

    if let Some(pending) = current.take() {
        entries.push(pending.finish(options));
    }

    entries
}

/// Map the delimiter offset within a logical line back to a physical offset.
fn delimiter_offset_in_physical(
    logical_line: &LogicalLine,
    lines: &[&str],
    indent: usize,
    eq_pos: usize,
) -> usize {
    if !logical_line.folded {
        // `eq_pos` is relative to the trimmed line, so re-add the indentation.
        return indent + eq_pos;
    }
    let last = lines[logical_line.end];
    let tail = last.trim();
    let tail_start = logical_line.text.len().saturating_sub(tail.len());
    if eq_pos >= tail_start {
        indent_of(last) + (eq_pos - tail_start)
    } else {
        // The delimiter sits inside the folded key text; fall back to the last
        // line's own delimiter so rendering stays within that line.
        last.find('=').unwrap_or_else(|| indent_of(last))
    }
}

struct PendingEntry {
    key: String,
    indent: usize,
    header: usize,
    start: usize,
    header_end: usize,
    end: usize,
    delimiter: Option<usize>,
    value_lines: Vec<String>,
}

impl PendingEntry {
    fn new(key: String, indent: usize, header: usize, logical_line: &LogicalLine) -> Self {
        Self {
            key,
            indent,
            header,
            start: logical_line.start,
            header_end: logical_line.end,
            end: logical_line.end,
            delimiter: None,
            value_lines: Vec::new(),
        }
    }

    fn finish(self, options: &Options) -> LexedEntry {
        let value = finalize_value(&self.value_lines.join("\n"), options);
        LexedEntry {
            key: self.key,
            value,
            indent: self.indent,
            header: self.header,
            start: self.start,
            header_end: self.header_end,
            end: self.end,
            delimiter: self.delimiter,
        }
    }
}

/// Lex a value block whose common indentation should be stripped first.
///
/// This is the `parse_indented` behavior from the CCL specification: the block
/// may be indented as a whole, so the shared prefix is removed before lexing.
#[cfg(feature = "unstable")]
pub(crate) fn lex_indented(input: &str, options: &Options) -> Vec<(String, String)> {
    let min_indent = input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(indent_of)
        .min()
        .unwrap_or(0);

    let dedented = input
        .lines()
        .map(|line| {
            let for_dedent = line.replace('\t', " ");
            if for_dedent.trim().is_empty() {
                options.process_tabs(line).into_owned()
            } else if for_dedent.len() > min_indent {
                if options.preserves_tabs() {
                    if line.len() > min_indent {
                        line[min_indent..].to_string()
                    } else {
                        line.trim_start().to_string()
                    }
                } else {
                    for_dedent[min_indent..].to_string()
                }
            } else if options.preserves_tabs() {
                line.trim_start().to_string()
            } else {
                for_dedent.trim_start().to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let entries_at_base = dedented
        .lines()
        .filter(|line| indent_of(line) == 0 && line.trim().contains('='))
        .count();

    if entries_at_base > 1 {
        lex_flat(&dedented, options)
    } else {
        lex_single_entry(&dedented, options)
    }
}

/// Lex every `key = value` pair as an independent entry, ignoring nesting.
#[cfg(feature = "unstable")]
fn lex_flat(input: &str, options: &Options) -> Vec<(String, String)> {
    let mut entries: Vec<(String, String)> = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let indent = indent_of(line);
        let trimmed = line.trim();

        if trimmed.contains('=') {
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim().to_string();
                let raw = &trimmed[eq_pos + 1..];
                let value = if options.is_strict_spacing() {
                    raw.trim_start_matches(' ')
                        .trim_end_matches(' ')
                        .to_string()
                } else {
                    raw.trim().to_string()
                };
                entries.push((key, value));
            }
        } else if indent > 0 && !entries.is_empty() {
            let last = entries.last_mut().expect("non-empty");
            if last.1.is_empty() {
                entries.push((trimmed.to_string(), String::new()));
            } else {
                last.1.push('\n');
                last.1.push_str(line);
            }
        } else if !entries.is_empty() {
            let last = entries.last_mut().expect("non-empty");
            if last.0.is_empty() {
                last.1.push('\n');
                last.1.push_str(trimmed);
            } else {
                entries.push((trimmed.to_string(), String::new()));
            }
        } else {
            entries.push((trimmed.to_string(), String::new()));
        }
    }

    entries
}

/// Lex the input as one entry whose value keeps its raw indentation.
#[cfg(feature = "unstable")]
fn lex_single_entry(input: &str, options: &Options) -> Vec<(String, String)> {
    let mut lines = input.lines();
    let first = lines.next().unwrap_or("");

    match first.find('=') {
        Some(eq_pos) => {
            let key = first[..eq_pos].trim().to_string();
            let inline = first[eq_pos + 1..].trim_start().to_string();
            let rest: Vec<&str> = lines.collect();
            let value = if rest.is_empty() {
                inline
            } else {
                let joined = if inline.trim().is_empty() {
                    format!("\n{}", rest.join("\n"))
                } else {
                    format!("{}\n{}", inline, rest.join("\n"))
                };
                options.process_tabs(&joined).into_owned()
            };
            vec![(key, value)]
        }
        None => vec![(first.trim().to_string(), String::new())],
    }
}

/// A flat `key = value` pair, without hierarchy.
///
/// Used by the spec-compliance surface in [`crate::unstable`].
#[cfg(feature = "unstable")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlatEntry {
    pub(crate) key: String,
    pub(crate) value: String,
}

/// Lex `input` into flat entries.
#[cfg(feature = "unstable")]
pub(crate) fn flat_entries(input: &str, options: &Options) -> Vec<FlatEntry> {
    let processed = options.process_crlf(input);
    let (lines, _) = physical_lines(&processed);
    lex(&lines, options)
        .into_iter()
        .map(|e| FlatEntry {
            key: e.key,
            value: e.value,
        })
        .collect()
}

/// Lex `input` into flat entries after stripping shared indentation.
#[cfg(feature = "unstable")]
pub(crate) fn flat_entries_indented(input: &str, options: &Options) -> Vec<FlatEntry> {
    let processed = options.process_crlf(input);
    lex_indented(&processed, options)
        .into_iter()
        .map(|(key, value)| FlatEntry { key, value })
        .collect()
}

/// Render flat entries back to CCL text.
#[cfg(feature = "unstable")]
pub(crate) fn print_flat(entries: &[FlatEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            if entry.key == "/" {
                format!("/= {}", entry.value)
            } else if entry.key.is_empty() {
                format!("= {}", entry.value)
            } else if entry.value.is_empty() || entry.value.starts_with('\n') {
                format!("{} ={}", entry.key, entry.value)
            } else {
                format!("{} = {}", entry.key, entry.value)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{SpacingBehavior, TabBehavior};

    fn lex_str(input: &str, options: &Options) -> Vec<LexedEntry> {
        let (lines, _) = physical_lines(input);
        lex(&lines, options)
    }

    #[test]
    fn splits_simple_entries() {
        let entries = lex_str("a = 1\nb = 2", &Options::new());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "a");
        assert_eq!(entries[0].value, "1");
        assert_eq!(entries[1].start, 1);
    }

    #[test]
    fn indented_lines_continue_the_entry_above() {
        let entries = lex_str("cfg =\n  host = h\n  port = 1", &Options::new());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, "\n  host = h\n  port = 1");
        assert_eq!(entries[0].start, 0);
        assert_eq!(entries[0].end, 2);
    }

    #[test]
    fn strict_spacing_rejects_unspaced_equals() {
        let strict = Options::new().with_spacing(SpacingBehavior::Strict);
        let entries = lex_str("a=1", &strict);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "a=1");
        assert_eq!(entries[0].value, "");
    }

    #[test]
    fn first_equals_wins_by_default() {
        let entries = lex_str("a=b=c", &Options::new());
        assert_eq!(entries[0].key, "a");
        assert_eq!(entries[0].value, "b=c");
    }

    #[test]
    fn tabs_can_be_converted() {
        let opts = Options::new().with_tabs(TabBehavior::ToSpaces);
        let entries = lex_str("a =\tvalue", &opts);
        assert_eq!(entries[0].value, "value");
    }

    #[test]
    fn delimiter_offset_maps_to_physical_line() {
        let entries = lex_str("  key = v", &Options::new());
        assert_eq!(entries[0].delimiter, Some(6));
        assert_eq!(&"  key = v"[6..7], "=");
    }

    #[test]
    fn print_flat_round_trips_entries() {
        let entries = flat_entries("name = Alice\nconfig =\n  port = 1", &Options::new());
        assert_eq!(print_flat(&entries), "name = Alice\nconfig =\n  port = 1");
    }

    #[test]
    fn comments_lex_as_slash_entries() {
        let entries = flat_entries("/= hello\na = 1", &Options::new());
        assert_eq!(entries[0].key, "/");
        assert_eq!(entries[0].value, "hello");
    }

    #[test]
    fn physical_lines_reports_trailing_newline() {
        let (lines, trailing) = physical_lines("a\nb\n");
        assert_eq!(lines, vec!["a", "b"]);
        assert!(trailing);
        let (lines, trailing) = physical_lines("a\nb");
        assert_eq!(lines, vec!["a", "b"]);
        assert!(!trailing);
    }
}
