#[cfg(test)]
#[macro_use]
extern crate quickcheck;
extern crate unicode_categories;

use std::borrow::Cow;
use std::{char, str};
use std::iter::Enumerate;
use unicode_categories::UnicodeCategories;

// escape performs some minimal 'shell-like' escaping on a given string
pub fn escape(s: &str) -> Cow<str> {
    let mut needs_escaping = false;
    let mut single_quotable = true;

    for c in s.chars() {
        if c == '\'' {
            single_quotable = false;
            needs_escaping = true;
        } else if c == '\\' {
            single_quotable = false;
            needs_escaping = true;
        } else if c == '"' {
            needs_escaping = true;
        } else if c.is_whitespace() {
            single_quotable = false;
            needs_escaping = true;
        } else if c.is_separator() {
            single_quotable = false;
            needs_escaping = true;
        } else if c.is_other() {
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
        // all characters should be fine for visual editing
        return format!("'{}'", s).into();
    }

    let mut output = String::with_capacity(s.len());
    output.push('"');

    for c in s.chars() {
        if c == '"' {
            output += "\\\"";
        } else if c == '\\' {
            output += "\\\\";
        } else if c == ' ' {
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

// TODO: this type signature is needlessly specific
fn parse_unicode(chars: &mut Enumerate<str::Chars>) -> Result<char, String> {
    match chars.next() {
        Some((_, '{')) => {}
        _ => {
            return Err("expected '{{' character in unicode escape".to_string());
        }
    }

    let unicode_seq: String = chars
        .take_while(|(_, c)| *c != '}')
        .map(|(_, c)| c)
        .collect();

    u32::from_str_radix(&unicode_seq, 16)
        .map_err(|e| format!("could not parse {} as u32 hex: {}", unicode_seq, e))
        .and_then(|u| char::from_u32(u).ok_or(format!("could not parse {} as a unicode char", u)))
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
