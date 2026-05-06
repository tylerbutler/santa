//! Pacman parser implementation for CCL.

use nom::bytes::complete::{tag, take_until};
use nom::character::complete::char;
use nom::sequence::terminated;
use nom::IResult;
use nom::Parser;

pub(crate) fn parse_first_equals_key(input: &str) -> IResult<&str, &str> {
    terminated(take_until("="), char('=')).parse(input)
}

pub(crate) fn parse_spaced_delimiter_key(input: &str) -> IResult<&str, &str> {
    terminated(take_until(" = "), tag(" = ")).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_equals_key_consumes_through_delimiter() {
        let (remaining, key) = parse_first_equals_key("name=value").unwrap();

        assert_eq!(key, "name");
        assert_eq!(remaining, "value");
    }

    #[test]
    fn first_equals_key_allows_multiline_keys() {
        let (remaining, key) = parse_first_equals_key("long\nkey = value").unwrap();

        assert_eq!(key, "long\nkey ");
        assert_eq!(remaining, " value");
    }

    #[test]
    fn spaced_delimiter_key_prefers_spaced_equals() {
        let (remaining, key) =
            parse_spaced_delimiter_key("https://example.com?q=1 = result").unwrap();

        assert_eq!(key, "https://example.com?q=1");
        assert_eq!(remaining, "result");
    }
}
