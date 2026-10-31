//! Structural document nodes for selective consumers.
//!
//! Decoding records fields without enforcing required fields, following references,
//! or discarding malformed siblings. A consumer decides which fields it needs to
//! admit. Valid structure has one typed owner; only invalid field values, literal
//! payloads and unknown vocabulary retain JSON values.
use crate::SchemaValue;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) mod metadata;
pub use metadata::MetadataPatch;

/// Presence and shape of one field. `Invalid(Null)` differs from `Missing`.
#[derive(Debug, Clone, Default)]
pub enum Field<T> {
    #[default]
    Missing,
    Value(T),
    Invalid(Value),
}

impl<T> Field<T> {
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
    pub fn as_value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            _ => None,
        }
    }
    pub fn as_value_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Value(value) => Some(value),
            _ => None,
        }
    }
    pub fn into_value(self) -> Option<T> {
        match self {
            Self::Value(value) => Some(value),
            _ => None,
        }
    }
}

impl<T: Serialize> Serialize for Field<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Value(value) => value.serialize(serializer),
            Self::Invalid(value) => value.serialize(serializer),
            Self::Missing => serializer.serialize_none(),
        }
    }
}

/// Move a value into its structural representation or return the unconsumed value.
/// This checks shape only; it does not establish OpenAPI validity.
pub trait Decode: Sized {
    fn decode(value: Value) -> Result<Self, Value>;
}

impl<T: Decode> Field<T> {
    pub fn from_value(value: Value) -> Self {
        match T::decode(value) {
            Ok(value) => Self::Value(value),
            Err(value) => Self::Invalid(value),
        }
    }
}

impl<'de, T: Decode> Deserialize<'de> for Field<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from_value(Value::deserialize(deserializer)?))
    }
}

impl Decode for String {
    fn decode(value: Value) -> Result<Self, Value> {
        match value {
            Value::String(value) => Ok(value),
            other => Err(other),
        }
    }
}
impl Decode for bool {
    fn decode(value: Value) -> Result<Self, Value> {
        match value {
            Value::Bool(value) => Ok(value),
            other => Err(other),
        }
    }
}
impl Decode for Value {
    fn decode(value: Value) -> Result<Self, Value> {
        Ok(value)
    }
}
impl<T: Decode> Decode for Box<T> {
    fn decode(value: Value) -> Result<Self, Value> {
        T::decode(value).map(Box::new)
    }
}
impl<T: Decode> Decode for Vec<Field<T>> {
    fn decode(value: Value) -> Result<Self, Value> {
        match value {
            Value::Array(values) => Ok(values.into_iter().map(Field::from_value).collect()),
            other => Err(other),
        }
    }
}
impl<T: Decode> Decode for BTreeMap<String, Field<T>> {
    fn decode(value: Value) -> Result<Self, Value> {
        match value {
            Value::Object(fields) => Ok(fields
                .into_iter()
                .map(|(key, value)| (key, Field::from_value(value)))
                .collect()),
            other => Err(other),
        }
    }
}
impl Decode for SchemaValue {
    fn decode(value: Value) -> Result<Self, Value> {
        if !value.is_object() && !value.is_boolean() {
            return Err(value);
        }
        Ok(Self::from_lossless_value(value).expect("object or boolean schema shape checked"))
    }
}

pub type Fields<T> = BTreeMap<String, Field<T>>;
pub type List<T> = Vec<Field<T>>;

// A single field declaration drives storage, decoding and serialization. Known
// fields never also live in extras. Collection entries decode independently.
macro_rules! node {
    (metadata_access $name:ident { $($field:ident : $ty:ty => $key:literal),* $(,)? }) => {
        node!(metadata $name { $($field : $ty => $key),* });
        impl $name {
            /// Borrow present metadata keys in lexical order.
            pub fn keyword_names(&self) -> Vec<&str> {
                let mut keys: Vec<&str> = [$(($key, !self.$field.is_missing()),)*]
                    .into_iter().filter_map(|(key, present)| present.then_some(key)).collect();
                keys.extend(self.extras.keys().map(String::as_str));
                keys.sort_unstable();
                keys.dedup();
                keys
            }
            /// Serialize one present field without constructing an intermediate value.
            pub fn serialize_keyword<S: Serializer>(&self, key: &str, serializer: S) -> Result<Option<S::Ok>, S::Error> {
                match key {
                    $($key if !self.$field.is_missing() => self.$field.serialize(serializer).map(Some),)*
                    _ => self.extras.get(key).map(|value| value.serialize(serializer)).transpose(),
                }
            }
        }
    };
    (metadata $name:ident { $($field:ident : $ty:ty => $key:literal),* $(,)? }) => {
        node!($name { $($field : $ty => $key),* });
        impl $name {
            /// Patch metadata fields in lexical order using the caller's null/literal policy.
            pub fn patch_metadata<P: MetadataPatch>(&mut self, patch: &Self, policy: &mut P) -> Result<(), P::Error> {
                let mut keys: Vec<&str> = [$(($key, !patch.$field.is_missing()),)*]
                    .into_iter().filter_map(|(key, present)| present.then_some(key)).collect();
                keys.extend(patch.extras.keys().map(String::as_str));
                keys.sort_unstable();
                keys.dedup();
                for key in keys {
                    self.patch_keyword_from(key, patch, policy)?;
                }
                Ok(())
            }

            /// Patch one present keyword, charging its key through the supplied policy.
            /// Absent keywords leave the target and policy untouched.
            pub fn patch_keyword_from<P: MetadataPatch>(&mut self, key: &str, patch: &Self, policy: &mut P) -> Result<(), P::Error> {
                match key {
                    $($key if !patch.$field.is_missing() => {
                        let literal = policy.key(key)?;
                        metadata::field(&mut self.$field, &patch.$field, literal, policy)?;
                        self.extras.remove(key);
                    },)*
                    _ => if let Some(value) = patch.extras.get(key) {
                        let literal = policy.key(key)?;
                        metadata::extra(&mut self.extras, key, value, literal, policy)?;
                    },
                }
                Ok(())
            }
        }
        impl metadata::MetadataValue for $name {
            fn patch<P: MetadataPatch>(&mut self, patch: &Self, policy: &mut P) -> Result<(), P::Error> {
                self.patch_metadata(patch, policy)
            }
        }
    };
    (reference $name:ident { $($field:ident : $ty:ty => $key:literal),* $(,)? }) => {
        node!($name { $($field : $ty => $key),* });
        impl $name {
            fn into_reference(self) -> Result<crate::Reference, serde_json::Error> {
                let mut node = self;
                let reference = admission::required(std::mem::take(&mut node.reference))?;
                let summary = admission::optional(std::mem::take(&mut node.summary))?;
                let description = admission::optional(std::mem::take(&mut node.description))?;
                let mut extras = node.extras;
                $(admission::retain(&mut extras, $key, node.$field)?;)*
                Ok(crate::Reference { _ref: reference, summary, description, extras })
            }
            fn only_reference_fields(&self) -> bool {
                self.extras.is_empty() && true $(&& (admission::is_reference_key($key) || self.$field.is_missing()))*
            }
        }
    };
    ($name:ident { $($field:ident : $ty:ty => $key:literal),* $(,)? }) => {
        #[derive(Debug, Clone, Default, Serialize)]
        pub struct $name {
            $(#[serde(rename = $key, skip_serializing_if = "Field::is_missing")]
            pub $field: Field<$ty>,)*
            #[serde(flatten)]
            pub extras: BTreeMap<String, Value>,
        }
        impl $name {
            /// Replace fields present in `siblings`, retaining absent target fields.
            /// This is a shallow document edit, with no reference semantics.
            pub fn overlay(&mut self, siblings: Self) {
                $(if !siblings.$field.is_missing() { self.$field = siblings.$field; })*
                self.extras.extend(siblings.extras);
            }

        }
        impl crate::HeapSize for $name {
            fn heap_size(&self) -> usize {
                crate::HeapSize::heap_size(&self.extras)
                    $(.saturating_add(crate::HeapSize::heap_size(&self.$field)))*
            }
        }
        impl Decode for $name {
            fn decode(value: Value) -> Result<Self, Value> {
                let Value::Object(mut fields) = value else { return Err(value); };
                Ok(Self {
                    $($field: fields.remove($key).map(Field::from_value).unwrap_or_default(),)*
                    extras: fields.into_iter().collect(),
                })
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Self::decode(Value::deserialize(deserializer)?)
                    .map_err(|_| serde::de::Error::custom(concat!("expected ", stringify!($name), " object")))
            }
        }
    }
}

pub mod asyncapi;

node!(metadata_access ExternalDocumentation { reference: String => "$ref", summary: String => "summary", description: String => "description", url: String => "url" });
node!(metadata Discriminator { property_name: String => "propertyName", mapping: Fields<String> => "mapping" });
node!(ServerVariable { values: List<String> => "enum", default: String => "default", description: String => "description" });
node!(Server { url: String => "url", description: String => "description", variables: Fields<ServerVariable> => "variables" });
node!(reference Example {
    reference: String => "$ref", summary: String => "summary", description: String => "description",
    value: Value => "value", external_value: String => "externalValue"
});
node!(reference Link {
    reference: String => "$ref", summary: String => "summary", description: String => "description",
    operation_ref: String => "operationRef", operation_id: String => "operationId",
    parameters: Fields<Value> => "parameters", request_body: Value => "requestBody", server: Server => "server"
});
node!(reference Header {
    reference: String => "$ref", summary: String => "summary", description: String => "description",
    required: bool => "required", deprecated: bool => "deprecated", allow_empty_value: bool => "allowEmptyValue",
    style: String => "style", explode: bool => "explode", allow_reserved: bool => "allowReserved",
    schema: SchemaValue => "schema", item_schema: SchemaValue => "itemSchema",
    example: Value => "example", examples: Fields<Example> => "examples", content: Fields<MediaType> => "content"
});
node!(reference Parameter {
    reference: String => "$ref", summary: String => "summary", name: String => "name", location: String => "in",
    description: String => "description", required: bool => "required", deprecated: bool => "deprecated",
    allow_empty_value: bool => "allowEmptyValue", style: String => "style", explode: bool => "explode",
    allow_reserved: bool => "allowReserved", schema: SchemaValue => "schema", item_schema: SchemaValue => "itemSchema",
    example: Value => "example", examples: Fields<Example> => "examples", content: Fields<MediaType> => "content"
});
node!(Encoding {
    content_type: String => "contentType", headers: Fields<Header> => "headers",
    style: String => "style", explode: bool => "explode", allow_reserved: bool => "allowReserved"
});
node!(MediaType {
    schema: SchemaValue => "schema", item_schema: SchemaValue => "itemSchema", example: Value => "example",
    examples: Fields<Example> => "examples", encoding: Fields<Encoding> => "encoding"
});
node!(reference RequestBody {
    reference: String => "$ref", summary: String => "summary", description: String => "description",
    required: bool => "required", content: Fields<MediaType> => "content"
});
node!(reference Response {
    reference: String => "$ref", summary: String => "summary", description: String => "description",
    headers: Fields<Header> => "headers", content: Fields<MediaType> => "content", links: Fields<Link> => "links"
});
node!(Operation {
    tags: List<String> => "tags", summary: String => "summary", description: String => "description",
    external_docs: ExternalDocumentation => "externalDocs", operation_id: String => "operationId",
    parameters: List<Parameter> => "parameters", request_body: RequestBody => "requestBody",
    responses: Responses => "responses", callbacks: Fields<Callback> => "callbacks",
    deprecated: bool => "deprecated", security: List<Fields<List<String>>> => "security", servers: List<Server> => "servers"
});
node!(PathItem {
    reference: String => "$ref", summary: String => "summary", description: String => "description",
    get: Box<Operation> => "get", put: Box<Operation> => "put", post: Box<Operation> => "post", delete: Box<Operation> => "delete",
    options: Box<Operation> => "options", head: Box<Operation> => "head", patch: Box<Operation> => "patch", trace: Box<Operation> => "trace",
    servers: List<Server> => "servers", parameters: List<Parameter> => "parameters"
});
node!(OauthFlow {
    authorization_url: String => "authorizationUrl", token_url: String => "tokenUrl", refresh_url: String => "refreshUrl",
    scopes: Fields<String> => "scopes"
});
node!(OauthFlows {
    implicit: OauthFlow => "implicit", password: OauthFlow => "password", client_credentials: OauthFlow => "clientCredentials",
    authorization_code: OauthFlow => "authorizationCode"
});
node!(SecurityScheme {
    reference: String => "$ref", summary: String => "summary", kind: String => "type", description: String => "description",
    name: String => "name", location: String => "in", scheme: String => "scheme", bearer_format: String => "bearerFormat",
    flows: OauthFlows => "flows", open_id_connect_url: String => "openIdConnectUrl"
});

/// Callback expressions are structural; extension values remain literal even if
/// an extension name contains braces.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Callback {
    #[serde(rename = "$ref", skip_serializing_if = "Field::is_missing")]
    pub reference: Field<String>,
    #[serde(flatten)]
    pub paths: Fields<PathItem>,
    #[serde(flatten)]
    pub extras: BTreeMap<String, Value>,
}

impl crate::HeapSize for Callback {
    fn heap_size(&self) -> usize {
        crate::HeapSize::heap_size(&self.reference)
            .saturating_add(crate::HeapSize::heap_size(&self.paths))
            .saturating_add(crate::HeapSize::heap_size(&self.extras))
    }
}

impl Decode for Callback {
    fn decode(value: Value) -> Result<Self, Value> {
        let Value::Object(mut fields) = value else {
            return Err(value);
        };
        let mut result = Self {
            reference: fields
                .remove("$ref")
                .map(Field::from_value)
                .unwrap_or_default(),
            ..Self::default()
        };
        for (key, value) in fields {
            if !key.starts_with("x-") && key.contains('{') {
                result.paths.insert(key, Field::from_value(value));
            } else {
                result.extras.insert(key, value);
            }
        }
        Ok(result)
    }
}

/// Response entries retain invalid siblings separately from valid responses.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Responses {
    #[serde(flatten)]
    pub data: Fields<Response>,
    #[serde(flatten)]
    pub extras: BTreeMap<String, Value>,
}
impl crate::HeapSize for Responses {
    fn heap_size(&self) -> usize {
        crate::HeapSize::heap_size(&self.data)
            .saturating_add(crate::HeapSize::heap_size(&self.extras))
    }
}

impl Decode for Responses {
    fn decode(value: Value) -> Result<Self, Value> {
        let Value::Object(fields) = value else {
            return Err(value);
        };
        let mut result = Self::default();
        for (key, value) in fields {
            if key.starts_with("x-") {
                result.extras.insert(key, value);
            } else {
                result.data.insert(key, Field::from_value(value));
            }
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    Schema,
    Response,
    Parameter,
    Example,
    RequestBody,
    Header,
    SecurityScheme,
    Link,
    Callback,
    PathItem,
}

impl ComponentKind {
    /// Identify a standard OpenAPI component collection without admitting its entries.
    pub fn from_group(group: &str) -> Option<Self> {
        match group {
            "schemas" => Some(Self::Schema),
            "responses" => Some(Self::Response),
            "parameters" => Some(Self::Parameter),
            "examples" => Some(Self::Example),
            "requestBodies" => Some(Self::RequestBody),
            "headers" => Some(Self::Header),
            "securitySchemes" => Some(Self::SecurityScheme),
            "links" => Some(Self::Link),
            "callbacks" => Some(Self::Callback),
            "pathItems" => Some(Self::PathItem),
            _ => None,
        }
    }
}

/// One selected component definition; the variant supplies its structural context.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Component {
    Schema(SchemaValue),
    Response(Box<Response>),
    Parameter(Box<Parameter>),
    Example(Box<Example>),
    RequestBody(Box<RequestBody>),
    Header(Box<Header>),
    SecurityScheme(Box<SecurityScheme>),
    Link(Box<Link>),
    Callback(Box<Callback>),
    PathItem(Box<PathItem>),
}
impl crate::HeapSize for Component {
    fn heap_size(&self) -> usize {
        match self {
            Self::Schema(value) => crate::HeapSize::heap_size(value),
            Self::Response(value) => crate::HeapSize::heap_size(value),
            Self::Parameter(value) => crate::HeapSize::heap_size(value),
            Self::Example(value) => crate::HeapSize::heap_size(value),
            Self::RequestBody(value) => crate::HeapSize::heap_size(value),
            Self::Header(value) => crate::HeapSize::heap_size(value),
            Self::SecurityScheme(value) => crate::HeapSize::heap_size(value),
            Self::Link(value) => crate::HeapSize::heap_size(value),
            Self::Callback(value) => crate::HeapSize::heap_size(value),
            Self::PathItem(value) => crate::HeapSize::heap_size(value),
        }
    }
}

impl Component {
    pub fn from_value(kind: ComponentKind, value: Value) -> Result<Self, Value> {
        match kind {
            ComponentKind::Schema => SchemaValue::decode(value).map(Self::Schema),
            ComponentKind::Response => Response::decode(value).map(Box::new).map(Self::Response),
            ComponentKind::Parameter => Parameter::decode(value).map(Box::new).map(Self::Parameter),
            ComponentKind::Example => Example::decode(value).map(Box::new).map(Self::Example),
            ComponentKind::RequestBody => RequestBody::decode(value)
                .map(Box::new)
                .map(Self::RequestBody),
            ComponentKind::Header => Header::decode(value).map(Box::new).map(Self::Header),
            ComponentKind::SecurityScheme => SecurityScheme::decode(value)
                .map(Box::new)
                .map(Self::SecurityScheme),
            ComponentKind::Link => Link::decode(value).map(Box::new).map(Self::Link),
            ComponentKind::Callback => Callback::decode(value).map(Box::new).map(Self::Callback),
            ComponentKind::PathItem => PathItem::decode(value).map(Box::new).map(Self::PathItem),
        }
    }
}

/// Borrow a structural schema without following references or visiting literals.
pub trait SchemaLookup {
    fn schema_at_tokens<T: AsRef<str>>(&self, tokens: &[T]) -> Option<&SchemaValue>;
}
impl SchemaLookup for SchemaValue {
    fn schema_at_tokens<T: AsRef<str>>(&self, tokens: &[T]) -> Option<&SchemaValue> {
        SchemaValue::schema_at_tokens(self, tokens)
    }
}
impl<T: SchemaLookup> SchemaLookup for Field<T> {
    fn schema_at_tokens<S: AsRef<str>>(&self, tokens: &[S]) -> Option<&SchemaValue> {
        self.as_value()?.schema_at_tokens(tokens)
    }
}
impl<T: SchemaLookup> SchemaLookup for Fields<T> {
    fn schema_at_tokens<S: AsRef<str>>(&self, tokens: &[S]) -> Option<&SchemaValue> {
        let (key, tail) = tokens.split_first()?;
        self.get(key.as_ref())?.schema_at_tokens(tail)
    }
}
macro_rules! schema_lookup {
    ($name:ident { $($key:literal => $field:ident),* $(,)? }) => {
        impl SchemaLookup for $name {
            fn schema_at_tokens<T: AsRef<str>>(&self, tokens: &[T]) -> Option<&SchemaValue> {
                let (key, tail) = tokens.split_first()?;
                match key.as_ref() { $($key => self.$field.schema_at_tokens(tail),)* _ => None }
            }
        }
    }
}
schema_lookup!(Response { "headers" => headers, "content" => content });
schema_lookup!(Parameter { "schema" => schema, "itemSchema" => item_schema, "content" => content });
schema_lookup!(Header { "schema" => schema, "itemSchema" => item_schema, "content" => content });
schema_lookup!(RequestBody { "content" => content });
schema_lookup!(MediaType { "schema" => schema, "itemSchema" => item_schema, "encoding" => encoding });
schema_lookup!(Encoding { "headers" => headers });
impl SchemaLookup for Component {
    fn schema_at_tokens<T: AsRef<str>>(&self, tokens: &[T]) -> Option<&SchemaValue> {
        match self {
            Self::Schema(value) => value.schema_at_tokens(tokens),
            Self::Response(value) => value.schema_at_tokens(tokens),
            Self::Parameter(value) => value.schema_at_tokens(tokens),
            Self::Header(value) => value.schema_at_tokens(tokens),
            Self::RequestBody(value) => value.schema_at_tokens(tokens),
            _ => None,
        }
    }
}

/// Borrowed structural views share one child inventory. Literal payloads are excluded.
macro_rules! node_view {
    ($module:ident, $view:ident, [$($mutability:tt)?], $node:ident, $reference:ident, $values:ident) => {
        mod $module {
            use super::*;
            pub enum $view<'a> {
                Schema(&'a $($mutability)? SchemaValue),
                Response(&'a $($mutability)? Response),
                Parameter(&'a $($mutability)? Parameter),
                Example(&'a $($mutability)? Example),
                RequestBody(&'a $($mutability)? RequestBody),
                Header(&'a $($mutability)? Header),
                SecurityScheme(&'a $($mutability)? SecurityScheme),
                Link(&'a $($mutability)? Link),
                Callback(&'a $($mutability)? Callback),
                PathItem(&'a $($mutability)? PathItem),
                Operation(&'a $($mutability)? Operation),
                Responses(&'a $($mutability)? Responses),
                MediaType(&'a $($mutability)? MediaType),
                Encoding(&'a $($mutability)? Encoding),
                Invalid(&'a $($mutability)? Value),
            }
            fn child<T: StructuralNode, E>(
                field: & $($mutability)? Field<T>,
                key: &str,
                visit: &mut impl FnMut(&str, $view<'_>) -> Result<(), E>,
            ) -> Result<(), E> {
                match field {
                    Field::Missing => Ok(()),
                    Field::Value(value) => visit(key, value.$node()),
                    Field::Invalid(value) => visit(key, $view::Invalid(value)),
                }
            }
            fn children<T: StructuralNode, E>(
                field: & $($mutability)? Field<Fields<T>>,
                key: &str,
                visit: &mut impl FnMut(&str, $view<'_>) -> Result<(), E>,
            ) -> Result<(), E> {
                match field {
                    Field::Missing => Ok(()),
                    Field::Invalid(value) => visit(key, $view::Invalid(value)),
                    Field::Value(values) => {
                        for value in values.$values() {
                            child(value, key, visit)?;
                        }
                        Ok(())
                    }
                }
            }
            fn elements<T: StructuralNode, E>(
                field: & $($mutability)? Field<List<T>>,
                key: &str,
                visit: &mut impl FnMut(&str, $view<'_>) -> Result<(), E>,
            ) -> Result<(), E> {
                match field {
                    Field::Missing => Ok(()),
                    Field::Invalid(value) => visit(key, $view::Invalid(value)),
                    Field::Value(values) => {
                        for value in values {
                            child(value, key, visit)?;
                        }
                        Ok(())
                    }
                }
            }
            impl $view<'_> {
                pub fn $reference(& $($mutability)? self) -> Option<& $($mutability)? Field<String>> {
                    match self {
                        Self::Response(v) => Some(& $($mutability)? v.reference),
                        Self::Parameter(v) => Some(& $($mutability)? v.reference),
                        Self::Example(v) => Some(& $($mutability)? v.reference),
                        Self::RequestBody(v) => Some(& $($mutability)? v.reference),
                        Self::Header(v) => Some(& $($mutability)? v.reference),
                        Self::SecurityScheme(v) => Some(& $($mutability)? v.reference),
                        Self::Link(v) => Some(& $($mutability)? v.reference),
                        Self::Callback(v) => Some(& $($mutability)? v.reference),
                        Self::PathItem(v) => Some(& $($mutability)? v.reference),
                        _ => None,
                    }
                }

                /// Visit immediate document children in lexical field/map order. Schema
                /// children are exposed by Schema's visitor; literal payloads are not children.
                /// The caller owns recursion, policy, depth and accounting.
                pub fn try_for_each_child<E>(
                    & $($mutability)? self,
                    mut visit: impl FnMut(&str, $view<'_>) -> Result<(), E>,
                ) -> Result<(), E> {
                    match self {
                        Self::Response(v) => {
                            children(& $($mutability)? v.content, "content", &mut visit)?;
                            children(& $($mutability)? v.headers, "headers", &mut visit)?;
                            children(& $($mutability)? v.links, "links", &mut visit)?;
                        }
                        Self::Parameter(v) => {
                            children(& $($mutability)? v.content, "content", &mut visit)?;
                            children(& $($mutability)? v.examples, "examples", &mut visit)?;
                            child(& $($mutability)? v.item_schema, "itemSchema", &mut visit)?;
                            child(& $($mutability)? v.schema, "schema", &mut visit)?;
                        }
                        Self::Header(v) => {
                            children(& $($mutability)? v.content, "content", &mut visit)?;
                            children(& $($mutability)? v.examples, "examples", &mut visit)?;
                            child(& $($mutability)? v.item_schema, "itemSchema", &mut visit)?;
                            child(& $($mutability)? v.schema, "schema", &mut visit)?;
                        }
                        Self::RequestBody(v) => children(& $($mutability)? v.content, "content", &mut visit)?,
                        Self::MediaType(v) => {
                            children(& $($mutability)? v.encoding, "encoding", &mut visit)?;
                            children(& $($mutability)? v.examples, "examples", &mut visit)?;
                            child(& $($mutability)? v.item_schema, "itemSchema", &mut visit)?;
                            child(& $($mutability)? v.schema, "schema", &mut visit)?;
                        }
                        Self::Encoding(v) => children(& $($mutability)? v.headers, "headers", &mut visit)?,
                        Self::Operation(v) => {
                            children(& $($mutability)? v.callbacks, "callbacks", &mut visit)?;
                            elements(& $($mutability)? v.parameters, "parameters", &mut visit)?;
                            child(& $($mutability)? v.request_body, "requestBody", &mut visit)?;
                            child(& $($mutability)? v.responses, "responses", &mut visit)?;
                        }
                        Self::PathItem(v) => {
                            child(& $($mutability)? v.delete, "delete", &mut visit)?;
                            child(& $($mutability)? v.get, "get", &mut visit)?;
                            child(& $($mutability)? v.head, "head", &mut visit)?;
                            child(& $($mutability)? v.options, "options", &mut visit)?;
                            elements(& $($mutability)? v.parameters, "parameters", &mut visit)?;
                            child(& $($mutability)? v.patch, "patch", &mut visit)?;
                            child(& $($mutability)? v.post, "post", &mut visit)?;
                            child(& $($mutability)? v.put, "put", &mut visit)?;
                            child(& $($mutability)? v.trace, "trace", &mut visit)?;
                        }
                        Self::Responses(v) => {
                            for (key, value) in & $($mutability)? v.data {
                                child(value, key, &mut visit)?;
                            }
                        }
                        Self::Callback(v) => {
                            for (key, value) in & $($mutability)? v.paths {
                                child(value, key, &mut visit)?;
                            }
                        }
                        _ => {}
                    }
                    Ok(())
                }
            }

        }
        pub use $module::$view;
    };
}
node_view!(borrowed, Node, [], node, reference, values);
node_view!(editable, NodeMut, [mut], node_mut, reference_mut, values_mut);

pub trait StructuralNode {
    fn node(&self) -> Node<'_>;
    fn node_mut(&mut self) -> NodeMut<'_>;
}
macro_rules! structural_nodes {
    ($($name:ident),*) => {
        $(impl StructuralNode for $name {
            fn node(&self) -> Node<'_> { Node::$name(self) }
            fn node_mut(&mut self) -> NodeMut<'_> { NodeMut::$name(self) }
        })*
        impl NodeMut<'_> {
            pub fn as_ref(&self) -> Node<'_> {
                match self {
                    $(Self::$name(value) => Node::$name(value),)*
                    Self::Schema(value) => Node::Schema(value),
                    Self::Invalid(value) => Node::Invalid(value),
                }
            }
        }
    };
}
structural_nodes!(
    Response,
    Parameter,
    Example,
    RequestBody,
    Header,
    SecurityScheme,
    Link,
    Callback,
    PathItem,
    Operation,
    Responses,
    MediaType,
    Encoding
);
impl<T: StructuralNode> StructuralNode for Box<T> {
    fn node(&self) -> Node<'_> {
        self.as_ref().node()
    }
    fn node_mut(&mut self) -> NodeMut<'_> {
        self.as_mut().node_mut()
    }
}
impl StructuralNode for SchemaValue {
    fn node(&self) -> Node<'_> {
        Node::Schema(self)
    }
    fn node_mut(&mut self) -> NodeMut<'_> {
        NodeMut::Schema(self)
    }
}
macro_rules! component_nodes {
    ($($name:ident),*) => {
        impl StructuralNode for Component {
            fn node(&self) -> Node<'_> {
                match self { $(Self::$name(value) => value.node(),)* }
            }
            fn node_mut(&mut self) -> NodeMut<'_> {
                match self { $(Self::$name(value) => value.node_mut(),)* }
            }
        }
    };
}
component_nodes!(
    Schema,
    Response,
    Parameter,
    Example,
    RequestBody,
    Header,
    SecurityScheme,
    Link,
    Callback,
    PathItem
);

mod admission;

impl Callback {
    pub fn overlay(&mut self, siblings: Self) {
        if !siblings.reference.is_missing() {
            self.reference = siblings.reference;
        }
        self.paths.extend(siblings.paths);
        self.extras.extend(siblings.extras);
    }
}
impl Component {
    /// Overlay present fields of the same component kind. A mismatched kind is
    /// returned unchanged, so callers retain ownership when rejecting the edit.
    pub fn overlay(&mut self, siblings: Self) -> Result<(), Self> {
        match (self, siblings) {
            (Self::Response(target), Self::Response(siblings)) => target.overlay(*siblings),
            (Self::Parameter(target), Self::Parameter(siblings)) => target.overlay(*siblings),
            (Self::Example(target), Self::Example(siblings)) => target.overlay(*siblings),
            (Self::RequestBody(target), Self::RequestBody(siblings)) => target.overlay(*siblings),
            (Self::Header(target), Self::Header(siblings)) => target.overlay(*siblings),
            (Self::SecurityScheme(target), Self::SecurityScheme(siblings)) => {
                target.overlay(*siblings)
            }
            (Self::Link(target), Self::Link(siblings)) => target.overlay(*siblings),
            (Self::Callback(target), Self::Callback(siblings)) => target.overlay(*siblings),
            (Self::PathItem(target), Self::PathItem(siblings)) => target.overlay(*siblings),
            (_, siblings) => return Err(siblings),
        }
        Ok(())
    }
}

impl PathItem {
    /// Borrow an operation field for an OpenAPI method. Unknown methods have no field.
    pub fn operation(&self, method: &str) -> Option<&Field<Box<Operation>>> {
        match method {
            "get" => Some(&self.get),
            "put" => Some(&self.put),
            "post" => Some(&self.post),
            "delete" => Some(&self.delete),
            "options" => Some(&self.options),
            "head" => Some(&self.head),
            "patch" => Some(&self.patch),
            "trace" => Some(&self.trace),
            _ => None,
        }
    }
}

mod upgrade;
