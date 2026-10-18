//! yq's `envsubst` (#4872): replace `$VAR`, `${VAR}` and the `${VAR:-default}` family in a string
//! with the environment, the way yq's `a8m/envsubst` does.
//!
//! Every rule below was captured from yq v4.53.3 against a fuzzed set of strings, not read from
//! the Go source.

#[cfg(not(test))]
use alloc::{string::String, vec::Vec};
#[cfg(test)]
use std::{string::String, vec::Vec};

/// The `envsubst(nu)`, `envsubst(ne)` and `envsubst(ff)` flags (`yq`'s `NoUnset`, `NoEmpty`,
/// `FailFast`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EnvsubstFlags {
    /// `nu`: an unset variable is an error.
    pub no_unset: bool,
    /// `ne`: a variable that is set but empty is an error.
    pub no_empty: bool,
    /// `ff`: stop at the first error instead of reporting every one.
    pub fail_fast: bool,
}

/// `input` with its variables replaced by what `lookup` answers (`None` is unset).
///
/// An error is the messages of every unset/empty variable the flags forbid, one per line (or just
/// the first with `fail_fast`), or a syntax error.
pub(crate) fn envsubst(
    input: &str,
    flags: EnvsubstFlags,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<String, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut errors: Vec<String> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '$' {
            out.push(c);
            i += 1;
            continue;
        }
        match chars.get(i + 1) {
            Some('$') => {
                out.push('$');
                i += 2;
            }
            // `${` not followed by a name (`${-}`, `${@}`, `${.}`) is text.
            Some('{')
                if !chars
                    .get(i + 2)
                    .is_some_and(|&n| n.is_alphanumeric() || matches!(n, '_' | ' ' | '#')) =>
            {
                out.push('$');
                i += 1;
            }
            Some('{') => {
                let (consumed, value) = braced(&chars[i + 2..], flags, lookup, &mut errors)?;
                out.push_str(&value);
                i += 2 + consumed;
            }
            Some(&n) if n.is_alphanumeric() => {
                let len = chars[i + 1..]
                    .iter()
                    .take_while(|&&c| is_name_char(c))
                    .count();
                let name: String = chars[i + 1..i + 1 + len].iter().collect();
                out.push_str(&variable(&name, flags, lookup, &mut errors));
                i += 1 + len;
            }
            _ => {
                out.push('$');
                i += 1;
            }
        }
        if flags.fail_fast && !errors.is_empty() {
            break;
        }
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors.join("\n"))
    }
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// A plain variable: its value, or "" and the errors the flags ask for.
fn variable(
    name: &str,
    flags: EnvsubstFlags,
    lookup: &dyn Fn(&str) -> Option<String>,
    errors: &mut Vec<String>,
) -> String {
    match lookup(name) {
        None => {
            if flags.no_unset {
                errors.push(alloc_format(name, "not set"));
            }
            String::new()
        }
        Some(v) if v.is_empty() => {
            if flags.no_empty {
                errors.push(alloc_format(name, "set but empty"));
            }
            v
        }
        Some(v) => v,
    }
}

fn alloc_format(name: &str, what: &str) -> String {
    let mut s = String::from("variable ${");
    s.push_str(name);
    s.push_str("} ");
    s.push_str(what);
    s
}

/// The `${...}` after its `${`: how many characters it consumed (closing brace included) and
/// what it stands for.
fn braced(
    rest: &[char],
    flags: EnvsubstFlags,
    lookup: &dyn Fn(&str) -> Option<String>,
    errors: &mut Vec<String>,
) -> Result<(usize, String), String> {
    let Some(close) = rest.iter().position(|&c| c == '}') else {
        return Err(String::from("closing brace expected"));
    };
    let body: &[char] = &rest[..close];
    let consumed = close + 1;
    if body.is_empty() {
        let mut literal = String::from("${}");
        return Ok((consumed, std_mem_take(&mut literal)));
    }
    // The name is the run of identifier characters (a leading blank or `#` is part of it, as yq
    // reads `${ FOO }` and `${#FOO}`); whatever follows that is not an operator is ignored, so
    // `${FOO^^}` and `${FOO:1}` are `$FOO`.
    let name_len = body
        .iter()
        .position(|&c| !(c.is_alphanumeric() || matches!(c, '_' | ' ' | '#')))
        .unwrap_or(body.len());
    let name: String = body[..name_len]
        .iter()
        .collect::<String>()
        .trim_end()
        .into();
    let operator: String = body[name_len..].iter().collect();
    let value = lookup(&name);
    let set = value.is_some();
    let non_empty = value.as_deref().is_some_and(|v| !v.is_empty());
    let text = |v: &Option<String>| v.clone().unwrap_or_default();
    let out = if let Some(default) = operator
        .strip_prefix(":-")
        .or_else(|| operator.strip_prefix(":="))
    {
        if non_empty {
            text(&value)
        } else {
            String::from(default)
        }
    } else if let Some(default) = operator
        .strip_prefix('-')
        .or_else(|| operator.strip_prefix('='))
    {
        if set {
            variable(&name, flags, lookup, errors)
        } else {
            String::from(default)
        }
    } else if let Some(alt) = operator
        .strip_prefix(":+")
        .or_else(|| operator.strip_prefix('+'))
    {
        if set {
            String::from(alt)
        } else {
            String::new()
        }
    } else {
        variable(&name, flags, lookup, errors)
    };
    Ok((consumed, out))
}

fn std_mem_take(s: &mut String) -> String {
    core::mem::take(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(input: &str, flags: EnvsubstFlags) -> Result<String, String> {
        envsubst(input, flags, &|name| match name {
            "FOO" => Some("bar".to_string()),
            "EMPTY" => Some(String::new()),
            "SP" => Some("a b".to_string()),
            _ => None,
        })
    }

    /// Every row is yq v4.53.3's answer (see the CLI test for the full table).
    #[test]
    fn substitutes_the_forms_yq_does_4872() {
        let none = EnvsubstFlags::default();
        for (input, expected) in [
            ("$FOO", "bar"),
            ("${FOO}x", "barx"),
            ("$FOOx", ""),
            ("x$FOO.bar", "xbar.bar"),
            ("${MISSING:-def}", "def"),
            ("${EMPTY:-def}", "def"),
            ("${EMPTY-def}", ""),
            ("${MISSING-def}", "def"),
            ("${MISSING:=def}", "def"),
            ("${FOO:+alt}", "alt"),
            ("${EMPTY:+alt}", "alt"),
            ("${MISSING:+alt}", ""),
            ("$$FOO", "$FOO"),
            ("$$", "$"),
            ("$ FOO", "$ FOO"),
            ("a$", "a$"),
            ("${FOO^^}", "bar"),
            ("${FOO:1}", "bar"),
            ("${-}", "${-}"),
            ("${}", "${}"),
            ("$_", "$_"),
            ("${SP}", "a b"),
        ] {
            assert_eq!(run(input, none).as_deref(), Ok(expected), "{input:?}");
        }
        assert_eq!(
            run("${FOO", none),
            Err("closing brace expected".to_string())
        );
    }

    #[test]
    fn the_flags_make_unset_and_empty_variables_errors_4872() {
        let nu = EnvsubstFlags {
            no_unset: true,
            ..Default::default()
        };
        let ne = EnvsubstFlags {
            no_empty: true,
            ..Default::default()
        };
        let both = EnvsubstFlags {
            no_unset: true,
            no_empty: true,
            fail_fast: false,
        };
        let first = EnvsubstFlags {
            fail_fast: true,
            ..both
        };
        assert_eq!(
            run("$MISSING", nu),
            Err("variable ${MISSING} not set".to_string())
        );
        assert_eq!(run("$EMPTY", nu).as_deref(), Ok(""));
        assert_eq!(
            run("$EMPTY", ne),
            Err("variable ${EMPTY} set but empty".to_string())
        );
        assert_eq!(
            run("${EMPTY-d}", ne),
            Err("variable ${EMPTY} set but empty".to_string())
        );
        assert_eq!(run("${EMPTY:-d}", ne).as_deref(), Ok("d"));
        assert_eq!(
            run("$EMPTY$MISSING", both),
            Err("variable ${EMPTY} set but empty\nvariable ${MISSING} not set".to_string())
        );
        assert_eq!(
            run("$EMPTY$MISSING", first),
            Err("variable ${EMPTY} set but empty".to_string())
        );
    }
}
