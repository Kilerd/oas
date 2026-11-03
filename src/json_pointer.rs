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

/// Where a child sits under its parent: the one or two reference tokens that
/// lead from the parent's pointer to the child's.
///
/// Keyed visitors (`try_for_each_child_at` on the structural nodes and on
/// [`Schema`](crate::Schema)) report a `Step` with every child, so a caller
/// can build the RFC 6901 pointer of any node during its own recursion. The
/// step borrows the field name and the map key; [`Display`](std::fmt::Display)
/// writes the escaped pointer suffix, and [`Step::push_tokens`] appends the
/// unescaped tokens to a token path.
///
/// ```
/// use oas::json_pointer::Step;
/// use oas::{Schema, SchemaValue};
///
/// fn pointers(schema: &SchemaValue, at: &str, found: &mut Vec<String>) {
///     found.push(at.to_owned());
///     if let Some(object) = schema.as_object() {
///         object
///             .try_for_each_child_at(|step, child| {
///                 pointers(child, &format!("{at}{step}"), found);
///                 Ok::<_, ()>(())
///             })
///             .unwrap();
///     }
/// }
///
/// let mut root = Schema::new();
/// root.properties = Some([("a/b".to_owned(), SchemaValue::from(true))].into());
/// root.all_of = Some(vec![SchemaValue::from(false)]);
/// let mut found = Vec::new();
/// pointers(&root.into(), "", &mut found);
/// assert_eq!(found, ["", "/allOf/0", "/properties/a~1b"]);
/// assert_eq!(Step::Entry { field: "properties", key: "a/b" }.to_string(), "/properties/a~1b");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step<'a> {
    /// `/name`: a declared field, or an entry of a node that is itself a map,
    /// such as an OpenAPI path, response status or callback expression.
    Token(&'a str),
    /// `/field/key`: an entry of a map-valued field.
    Entry { field: &'a str, key: &'a str },
    /// `/field/index`: an element of an array-valued field.
    Element { field: &'a str, index: usize },
}

impl<'a> Step<'a> {
    /// The first token, unescaped. This is the name the unkeyed
    /// `try_for_each_child` visitors report for the same child.
    pub fn name(&self) -> &'a str {
        match *self {
            Self::Token(name) => name,
            Self::Entry { field, .. } | Self::Element { field, .. } => field,
        }
    }

    /// The number of reference tokens: 1 for [`Step::Token`], otherwise 2.
    pub fn token_count(&self) -> usize {
        match self {
            Self::Token(_) => 1,
            Self::Entry { .. } | Self::Element { .. } => 2,
        }
    }

    /// Append the unescaped tokens to a token path and return how many were
    /// appended, so the caller can truncate the path after descending.
    pub fn push_tokens(&self, tokens: &mut Vec<String>) -> usize {
        match *self {
            Self::Token(name) => tokens.push(name.to_owned()),
            Self::Entry { field, key } => tokens.extend([field.to_owned(), key.to_owned()]),
            Self::Element { field, index } => tokens.extend([field.to_owned(), index.to_string()]),
        }
        self.token_count()
    }
}

/// Writes the escaped pointer suffix, such as `/properties/a~1b` or `/allOf/0`.
impl std::fmt::Display for Step<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Token(name) => write!(formatter, "/{}", escape(name)),
            Self::Entry { field, key } => write!(formatter, "/{}/{}", escape(field), escape(key)),
            Self::Element { field, index } => write!(formatter, "/{}/{index}", escape(field)),
        }
    }
}
