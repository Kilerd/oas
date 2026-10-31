//! Consuming admission from lossless operation fields to the ordinary model.
use super::*;
use serde::de::Error as _;
use serde::Deserialize;
use serde_json::Error;

type Result<T> = std::result::Result<T, Error>;

fn invalid() -> Error {
    Error::custom("invalid or missing operation field")
}

// Checking is needed only to select the inline candidate of a reference with
// siblings. The ordinary path consumes its fields directly.
pub(super) trait Admit: Sized {
    type Output;
    fn check(&self) -> Result<()>;
    fn admit(self) -> Result<Self::Output>;
}

macro_rules! identity {
    ($($ty:ty),* $(,)?) => { $(impl Admit for $ty {
        type Output = Self;
        fn check(&self) -> Result<()> { Ok(()) }
        fn admit(self) -> Result<Self> { Ok(self) }
    })* };
}
identity!(String, bool, Value);

impl Admit for SchemaValue {
    type Output = Self;
    fn check(&self) -> Result<()> {
        self.check_legacy_root()
    }
    fn admit(self) -> Result<Self> {
        self.admit_legacy_root()
    }
}

impl<T: Admit> Admit for Box<T> {
    type Output = T::Output;
    fn check(&self) -> Result<()> {
        self.as_ref().check()
    }
    fn admit(self) -> Result<Self::Output> {
        (*self).admit()
    }
}

impl<T: Admit> Admit for List<T> {
    type Output = Vec<T::Output>;
    fn check(&self) -> Result<()> {
        self.iter().try_for_each(required_check)
    }
    fn admit(self) -> Result<Self::Output> {
        self.into_iter().map(required).collect()
    }
}

impl<T: Admit> Admit for Fields<T> {
    type Output = BTreeMap<String, T::Output>;
    fn check(&self) -> Result<()> {
        self.values().try_for_each(required_check)
    }
    fn admit(self) -> Result<Self::Output> {
        self.into_iter()
            .map(|(key, value)| Ok((key, required(value)?)))
            .collect()
    }
}

fn required_check<T: Admit>(field: &Field<T>) -> Result<()> {
    match field {
        Field::Value(value) => value.check(),
        _ => Err(invalid()),
    }
}
pub(super) fn required<T: Admit>(field: Field<T>) -> Result<T::Output> {
    match field {
        Field::Value(value) => value.admit(),
        _ => Err(invalid()),
    }
}
fn optional_check<T: Admit>(field: &Field<T>) -> Result<()> {
    match field {
        Field::Missing | Field::Invalid(Value::Null) => Ok(()),
        _ => required_check(field),
    }
}
pub(super) fn optional<T: Admit>(field: Field<T>) -> Result<Option<T::Output>> {
    match field {
        Field::Missing | Field::Invalid(Value::Null) => Ok(None),
        _ => required(field).map(Some),
    }
}
fn security(
    field: Field<List<Fields<List<String>>>>,
) -> Result<Option<Vec<crate::SecurityRequirement>>> {
    optional(field).map(|values| {
        values.map(|values| {
            values
                .into_iter()
                .map(|data| crate::SecurityRequirement { data })
                .collect()
        })
    })
}
fn present_check(_: &Field<Value>) -> Result<()> {
    Ok(())
}
fn present(field: Field<Value>) -> Result<Option<Value>> {
    Ok(match field {
        Field::Missing => None,
        Field::Value(value) | Field::Invalid(value) => Some(value),
    })
}

pub(super) fn is_reference_key(key: &str) -> bool {
    matches!(key, "$ref" | "summary" | "description")
}

// These fields become opaque in the strict model (reference siblings or fields
// it does not model). There is no decoding of the resulting JSON values.
pub(super) fn retain<T: Serialize>(
    extras: &mut BTreeMap<String, Value>,
    key: &str,
    field: Field<T>,
) -> Result<()> {
    let value = match field {
        Field::Missing => return Ok(()),
        Field::Invalid(value) => value,
        Field::Value(value) => serde_json::to_value(value)?,
    };
    extras.insert(key.into(), value);
    Ok(())
}

macro_rules! check_field {
    (required, $field:expr) => {
        required_check($field)
    };
    (optional, $field:expr) => {
        optional_check($field)
    };
    (security, $field:expr) => {
        optional_check($field)
    };
    (present, $field:expr) => {
        present_check($field)
    };
}

// Rust field mappings construct the existing strict structs; the lossless node
// declaration remains the source of wire keys used for reference siblings.
macro_rules! inline {
    ($name:ident { $($target:ident: $rule:ident($source:ident)),* $(,)? }
     extras |$node:ident| $extras:block) => {
        impl $name {
            fn check_inline(&self) -> Result<()> {
                $(check_field!($rule, &self.$source)?;)*
                Ok(())
            }
            fn admit_inline(self) -> Result<crate::$name> {
                let mut $node = self;
                let extras = $extras;
                Ok(crate::$name {
                    $($target: $rule($node.$source)?,)*
                    extras,
                })
            }
        }
    };
}
macro_rules! plain {
    ($name:ident) => {
        impl Admit for $name {
            type Output = crate::$name;
            fn check(&self) -> Result<()> {
                self.check_inline()
            }
            fn admit(self) -> Result<Self::Output> {
                self.admit_inline()
            }
        }
    };
}

macro_rules! referenceable {
    ($name:ident) => {
        impl $name {
            fn valid_reference(&self) -> bool {
                matches!(self.reference, Field::Value(_))
                    && optional_check(&self.summary).is_ok()
                    && optional_check(&self.description).is_ok()
            }
        }
        impl Admit for $name {
            type Output = crate::Referenceable<crate::$name>;
            fn check(&self) -> Result<()> {
                if self.valid_reference() {
                    Ok(())
                } else {
                    self.check_inline()
                }
            }
            fn admit(self) -> Result<Self::Output> {
                if self.valid_reference()
                    && (self.only_reference_fields() || self.check_inline().is_err())
                {
                    self.into_reference().map(crate::Referenceable::Reference)
                } else {
                    self.admit_inline().map(crate::Referenceable::Data)
                }
            }
        }
    };
}

inline!(Operation {
    tags: optional(tags), summary: optional(summary), description: optional(description),
    external_docs: optional(external_docs), operation_id: optional(operation_id),
    parameters: optional(parameters), request_body: optional(request_body),
    responses: required(responses), callbacks: optional(callbacks),
    deprecated: optional(deprecated), security: security(security), servers: optional(servers)
} extras |node| { std::mem::take(&mut node.extras) });
plain!(Operation);

impl Operation {
    /// Consume this selected operation using the ordinary model's admission rules.
    /// Typed children move into the result; schema fields cross their legacy
    /// admission boundary without a serialization/deserialization round trip.
    pub fn admit(self) -> Result<crate::Operation> {
        self.admit_inline()
    }
}

inline!(Server {
    url: required(url), description: optional(description), variables: optional(variables)
} extras |node| { std::mem::take(&mut node.extras) });
plain!(Server);
inline!(ServerVariable {
    _enum: optional(values), default: required(default), description: optional(description)
} extras |node| { std::mem::take(&mut node.extras) });
plain!(ServerVariable);
inline!(ExternalDocumentation {
    description: optional(description), url: required(url)
} extras |node| {
    retain(&mut node.extras, "$ref", std::mem::take(&mut node.reference))?;
    retain(&mut node.extras, "summary", std::mem::take(&mut node.summary))?;
    std::mem::take(&mut node.extras)
});
plain!(ExternalDocumentation);

inline!(PathItem {
    _ref: optional(reference), summary: optional(summary), description: optional(description),
    get: optional(get), put: optional(put), post: optional(post), delete: optional(delete),
    options: optional(options), head: optional(head), patch: optional(patch), trace: optional(trace),
    servers: optional(servers), parameters: optional(parameters)
} extras |node| { std::mem::take(&mut node.extras) });
plain!(PathItem);

inline!(Response {
    description: required(description), headers: optional(headers), content: optional(content),
    links: optional(links)
} extras |node| {
    retain(&mut node.extras, "$ref", std::mem::take(&mut node.reference))?;
    retain(&mut node.extras, "summary", std::mem::take(&mut node.summary))?;
    std::mem::take(&mut node.extras)
});
referenceable!(Response);

inline!(RequestBody {
    description: optional(description), required: optional(required), content: required(content)
} extras |node| {
    retain(&mut node.extras, "$ref", std::mem::take(&mut node.reference))?;
    retain(&mut node.extras, "summary", std::mem::take(&mut node.summary))?;
    std::mem::take(&mut node.extras)
});
referenceable!(RequestBody);

inline!(Header {
    description: optional(description), required: optional(required), deprecated: optional(deprecated),
    allow_empty_value: optional(allow_empty_value), style: optional(style), explode: optional(explode),
    allow_reserved: optional(allow_reserved), schema: optional(schema), example: present(example),
    examples: optional(examples), content: optional(content)
} extras |node| {
    retain(&mut node.extras, "$ref", std::mem::take(&mut node.reference))?;
    retain(&mut node.extras, "summary", std::mem::take(&mut node.summary))?;
    retain(&mut node.extras, "itemSchema", std::mem::take(&mut node.item_schema))?;
    std::mem::take(&mut node.extras)
});
referenceable!(Header);

inline!(MediaType {
    schema: optional(schema), item_schema: optional(item_schema), example: present(example),
    examples: optional(examples), encoding: optional(encoding)
} extras |node| { std::mem::take(&mut node.extras) });
plain!(MediaType);
inline!(Encoding {
    content_type: optional(content_type), headers: optional(headers), style: optional(style),
    explode: optional(explode), allow_reserved: optional(allow_reserved)
} extras |node| { std::mem::take(&mut node.extras) });
plain!(Encoding);
inline!(Example {
    summary: optional(summary), description: optional(description), value: present(value),
    external_value: optional(external_value)
} extras |node| {
    retain(&mut node.extras, "$ref", std::mem::take(&mut node.reference))?;
    std::mem::take(&mut node.extras)
});
referenceable!(Example);
inline!(Link {
    operation_ref: optional(operation_ref), operation_id: optional(operation_id),
    parameters: optional(parameters), request_body: present(request_body),
    description: optional(description), server: optional(server)
} extras |node| {
    retain(&mut node.extras, "$ref", std::mem::take(&mut node.reference))?;
    retain(&mut node.extras, "summary", std::mem::take(&mut node.summary))?;
    std::mem::take(&mut node.extras)
});
referenceable!(Link);

impl Parameter {
    fn location(&self) -> Result<crate::ParameterIn> {
        match &self.location {
            Field::Invalid(value) => crate::ParameterIn::deserialize(value),
            Field::Value(value) => crate::ParameterIn::deserialize(
                serde::de::value::StrDeserializer::<Error>::new(value),
            ),
            Field::Missing => Err(invalid()),
        }
    }
    fn check_inline(&self) -> Result<()> {
        required_check(&self.name)?;
        self.location()?;
        optional_check(&self.description)?;
        optional_check(&self.required)?;
        optional_check(&self.deprecated)?;
        optional_check(&self.allow_empty_value)?;
        optional_check(&self.style)?;
        optional_check(&self.explode)?;
        optional_check(&self.allow_reserved)?;
        optional_check(&self.schema)?;
        optional_check(&self.examples)?;
        optional_check(&self.content)
    }
    fn admit_inline(mut self) -> Result<crate::Parameter> {
        let location = self.location()?;
        retain(&mut self.extras, "$ref", self.reference)?;
        retain(&mut self.extras, "summary", self.summary)?;
        retain(&mut self.extras, "itemSchema", self.item_schema)?;
        Ok(crate::Parameter {
            name: required(self.name)?,
            _in: location,
            description: optional(self.description)?,
            required: optional(self.required)?,
            deprecated: optional(self.deprecated)?,
            allow_empty_value: optional(self.allow_empty_value)?,
            style: optional(self.style)?,
            explode: optional(self.explode)?,
            allow_reserved: optional(self.allow_reserved)?,
            schema: optional(self.schema)?,
            example: present(self.example)?,
            examples: optional(self.examples)?,
            content: optional(self.content)?,
            extras: self.extras,
        })
    }
}
referenceable!(Parameter);

impl Admit for Responses {
    type Output = crate::Responses;
    fn check(&self) -> Result<()> {
        for (key, value) in &self.data {
            if key == "default" {
                optional_check(value)?;
            } else if crate::types::is_response_status(key) {
                required_check(value)?;
            }
        }
        Ok(())
    }
    fn admit(self) -> Result<Self::Output> {
        let mut result = crate::Responses {
            default: None,
            data: BTreeMap::new(),
            extras: self.extras,
        };
        for (key, value) in self.data {
            if key == "default" {
                result.default = optional(value)?;
            } else if crate::types::is_response_status(&key) {
                result.data.insert(key, required(value)?);
            } else {
                retain(&mut result.extras, &key, value)?;
            }
        }
        Ok(result)
    }
}

impl Callback {
    fn valid_reference(&self) -> bool {
        matches!(self.reference, Field::Value(_))
            && ["summary", "description"].iter().all(|key| {
                matches!(
                    self.extras.get(*key),
                    None | Some(Value::Null | Value::String(_))
                )
            })
    }
}
impl Admit for Callback {
    type Output = crate::Referenceable<crate::Callback>;
    fn check(&self) -> Result<()> {
        if self.valid_reference() {
            Ok(())
        } else {
            self.paths.check()
        }
    }
    fn admit(self) -> Result<Self::Output> {
        let pure = self.paths.is_empty() && self.extras.keys().all(|key| is_reference_key(key));
        if self.valid_reference() && (pure || self.paths.check().is_err()) {
            let mut extras = self.extras;
            let summary = optional(Field::<String>::from_value(
                extras.remove("summary").unwrap_or(Value::Null),
            ))?;
            let description = optional(Field::<String>::from_value(
                extras.remove("description").unwrap_or(Value::Null),
            ))?;
            for (key, value) in self.paths {
                retain(&mut extras, &key, value)?;
            }
            Ok(crate::Referenceable::Reference(crate::Reference {
                _ref: required(self.reference)?,
                summary,
                description,
                extras,
            }))
        } else {
            let mut extras = self.extras;
            retain(&mut extras, "$ref", self.reference)?;
            Ok(crate::Referenceable::Data(crate::Callback {
                data: self.paths.admit()?,
                extras,
            }))
        }
    }
}
