#[cfg(test)]
#[macro_use]
extern crate quickcheck;
extern crate unicode_categories;

use std::borrow::Cow;
use std::{char, str};
use unicode_categories::UnicodeCategories;

/// Escape the provided string with shell-like quoting and escapes.
/// Strings which do not need to be escaped will be returned unchanged.
///
/// # Examples
/// ```
/// use snailquote::escape;
/// # // The println/assert duplication is because I want to show the output you'd get without
/// # // rust's string quoting/escaping getting in the way
/// # // Ideally we could just assert on stdout, not duplicate, see
/// # // https://github.com/rust-lang/rfcs/issues/2270
/// println!("{}", escape("foo")); // no escapes needed
/// // foo
/// # assert_eq!(escape("foo"), "foo");
/// println!("{}", escape("String with spaces"));
/// // "String with spaces"
/// # assert_eq!(escape("String with spaces"), "\"String with spaces\"");
/// ```
// escape performs some minimal 'shell-like' escaping on a given string
pub fn escape(s: &str) -> Cow<str> {
    let mut needs_escaping = false;
    let mut single_quotable = true;

    for c in s.chars() {
        if c == '\'' || c == '\\' {
            single_quotable = false;
            needs_escaping = true;
        } else if c == '"' {
            needs_escaping = true;
        } else if c.is_whitespace() || c.is_separator() || c.is_other() {
            single_quotable = false;
            needs_escaping = true;
        }
        if needs_escaping && !single_quotable {
            break;
        }
    }

    if !needs_escaping {
        return Cow::from(s);
    }
    if single_quotable {
        return format!("'{}'", s).into();
    }
    // otherwise we need to double quote it

    let mut output = String::with_capacity(s.len());
    output.push('"');

    for c in s.chars() {
        if c == '"' {
            output += "\\\"";
        } else if c == '\\' {
            output += "\\\\";
        } else if c == ' ' {
            // avoid 'escape_unicode' for ' ' even though it's a separator
            output.push(c);
        } else if c.is_other() || c.is_separator() {
            output += &c.escape_unicode().to_string();
        } else {
            output.push(c);
        }
    }

    output.push('"');
    output.into()
}

/// Parse the provided string which is either a shell-like quoted string, or was produced with
/// [escape](escape).
///
/// # Details
///
/// Unescape is able to handle single quotes (which cannot contain any additional escapes), double
/// quotes (which may contain a set of escapes similar to ANSI-C, i.e. '\n', '\r', '\'', etc.
/// Unescape will also parse unicode escapes of the form "\u{01ff}". See
/// [char::escape_unicode](std::char::EscapeUnicode) in the Rust standard library for more
/// information on these escapes.
///
/// Multiple different quoting styles may be used in one string, for example, the following string
/// is valid: `'some spaces'_some_unquoted_"and a \t tab"`.
///
/// The returned result will contain a human readable error if the string cannot be parsed as a
/// valid quoted string.
///
/// # Examples
/// ```
/// use snailquote::unescape;
/// # // The println/assert duplication is because I want to show the output you'd get without
/// # // rust's string quoting/escaping getting in the way
/// # // Ideally we could just assert on stdout, not duplicate, see
/// # // https://github.com/rust-lang/rfcs/issues/2270
/// println!("{}", unescape("foo").unwrap());
/// // foo
/// # assert_eq!(unescape("foo").unwrap(), "foo");
/// println!("{}", unescape("'String with spaces'").unwrap());
/// // String with spaces
/// # assert_eq!(unescape("'String with spaces'").unwrap(), "String with spaces");
/// println!("{}", unescape("\"new\\nline\"").unwrap());
/// // new
/// // line
/// # assert_eq!(unescape("\"new\\nline\"").unwrap(), "new\nline");
/// println!("{}", unescape("'some spaces'_some_unquoted_\"and a \\t tab\"").unwrap());
/// // some spaces_some_unquoted_and a 	 tab
/// # assert_eq!(unescape("'some spaces'_some_unquoted_\"and a \\t tab\"").unwrap(), "some spaces_some_unquoted_and a \t tab");
/// ```
// TODO: more proper error type
pub fn unescape(s: &str) -> Result<String, String> {
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    let mut chars = s.chars().enumerate();

    let mut res = String::with_capacity(s.len());

    while let Some((idx, c)) = chars.next() {
        // when in a single quote, no escapes are possible
        if in_single_quote {
            if c == '\'' {
                in_single_quote = false;
                continue;
            }
        } else if in_double_quote {
            if c == '"' {
                in_double_quote = false;
                continue;
            }

            if c == '\\' {
                match chars.next() {
                    None => {
                        return Err(format!("invalid escape at char {} in string {}", idx, s));
                    }
                    Some((idx, c2)) => {
                        res.push(match c2 {
                            'a' => '\u{07}',
                            'b' => '\u{08}',
                            'v' => '\u{0B}',
                            'f' => '\u{0C}',
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            'e' | 'E' => '\u{1B}',
                            '\\' => '\\',
                            '\'' => '\'',
                            '"' => '"',
                            ' ' => ' ',
                            'u' => match parse_unicode(&mut chars) {
                                Ok(c) => c,
                                Err(e) => {
                                    return Err(format!(
                                        "\\u could not be parsed at {} in {}: {}",
                                        idx, s, e
                                    ));
                                }
                            },
                            _ => {
                                return Err(format!(
                                    "invalid escape {}{} at {} in {}",
                                    c, c2, idx, s
                                ));
                            }
                        });
                        continue;
                    }
                };
            }
        } else if c == '\'' {
            in_single_quote = true;
            continue;
        } else if c == '"' {
            in_double_quote = true;
            continue;
        }

        res.push(c);
    }

    Ok(res)
}

// parse_unicode takes an iterator over characters and attempts to extract a single unicode
// character from it.
// It parses escapes of the form '\u{65b9}', but this internal helper function expects the cursor
// to be advanced to between the 'u' and '{'.
// It also expects to be passed an iterator which includes the index for the purpose of advancing
// it  as well, such as is produced by enumerate.
fn parse_unicode<I>(chars: &mut I) -> Result<char, String>
where
    I: Iterator<Item = (usize, char)>,
{
    match chars.next() {
        Some((_, '{')) => {}
        _ => {
            return Err("expected '{{' character in unicode escape".to_string());
        }
    }

    let unicode_seq: String = chars
        .take_while(|&(_, c)| c != '}')
        .map(|(_, c)| c)
        .collect();

    u32::from_str_radix(&unicode_seq, 16)
        .map_err(|e| format!("could not parse {} as u32 hex: {}", unicode_seq, e))
        .and_then(|u| {
            char::from_u32(u).ok_or_else(|| format!("could not parse {} as a unicode char", u))
        })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_escape() {
        let test_cases = vec!["東方", "東方", "\"'", r#""\"'""#, "\\", "\"\\\\\""];

        for case in test_cases.chunks(2) {
            assert_eq!(escape(case[0]), case[1].to_string());
        }
    }

    #[test]
    fn test_unescape() {
        assert_eq!(unescape("\"\\u{6771}\\u{65b9}\""), Ok("東方".to_string()));
        assert_eq!(unescape("東方"), Ok("東方".to_string()));
        assert_eq!(unescape("\"\\\\\"'\"\"'"), Ok("\\\"\"".to_string()));
        assert_eq!(unescape("'\"'"), Ok("\"".to_string()));
        assert_eq!(unescape("'\"'"), Ok("\"".to_string()));
    }

    #[test]
    fn test_round_trip() {
        let test_cases = vec![
            "東方",
            "foo bar baz",
            "\\",
            "\0",
            "\"'",
            "\"'''''\"()())}{{}{}{{{!////",
        ];

        for case in test_cases {
            assert_eq!(unescape(&escape(case)), Ok(case.to_owned()));
        }
    }

    quickcheck! {
        fn round_trips(s: String) -> bool {
            s == unescape(&escape(&s)).unwrap()
        }
    }
}
