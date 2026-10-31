//! RFC 6901 string-form JSON Pointers. URI fragments must be decoded by the caller.
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidPointer;

impl std::fmt::Display for InvalidPointer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("invalid JSON Pointer")
    }
}
impl std::error::Error for InvalidPointer {}

/// Decode a string-form pointer. The empty string denotes the whole document;
/// empty tokens are significant. This does not follow references or interpret indices.
pub fn tokens(pointer: &str) -> Result<Vec<String>, InvalidPointer> {
    if pointer.is_empty() {
        return Ok(Vec::new());
    }
    pointer
        .strip_prefix('/')
        .ok_or(InvalidPointer)?
        .split('/')
        .map(decode_token)
        .collect()
}

fn decode_token(token: &str) -> Result<String, InvalidPointer> {
    let mut decoded = String::with_capacity(token.len());
    let mut chars = token.chars();
    while let Some(character) = chars.next() {
        decoded.push(match character {
            '~' => match chars.next() {
                Some('0') => '~',
                Some('1') => '/',
                _ => return Err(InvalidPointer),
            },
            other => other,
        });
    }
    Ok(decoded)
}

/// Escape one reference token, borrowing tokens that need no escaping.
pub fn escape(token: &str) -> Cow<'_, str> {
    if token.contains(['~', '/']) {
        Cow::Owned(token.replace('~', "~0").replace('/', "~1"))
    } else {
        Cow::Borrowed(token)
    }
}
