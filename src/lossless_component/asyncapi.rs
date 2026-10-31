//! Lossless AsyncAPI structural fields for selective document consumers.
//! Decoding establishes shape, not protocol validity or reference permission.
use super::*;

node!(metadata_access Components {
    parameters: Fields<Parameter> => "parameters", correlation_ids: Fields<Correlation> => "correlationIds",
    channel_bindings: Fields<Bindings> => "channelBindings", operation_bindings: Fields<Bindings> => "operationBindings",
    message_bindings: Fields<Bindings> => "messageBindings", tags: Fields<Tag> => "tags",
    external_docs: Fields<ExternalDocumentation> => "externalDocs"
});

impl NodeLookup for Components {
    fn node_at_tokens<T: AsRef<str>>(&self, tokens: &[T]) -> Option<Node<'_>> {
        let (key, tail) = tokens.split_first()?;
        match key.as_ref() {
            "parameters" => self.parameters.node_at_tokens(tail),
            "correlationIds" => self.correlation_ids.node_at_tokens(tail),
            "channelBindings" => self.channel_bindings.node_at_tokens(tail),
            "operationBindings" => self.operation_bindings.node_at_tokens(tail),
            "messageBindings" => self.message_bindings.node_at_tokens(tail),
            "tags" => self.tags.node_at_tokens(tail),
            "externalDocs" => self.external_docs.node_at_tokens(tail),
            key => raw_at_tokens(self.extras.get(key)?, tail).map(Node::Raw),
        }
    }
}

node!(metadata_access Metadata {
    tags: List<Tag> => "tags", external_docs: ExternalDocumentation => "externalDocs",
    bindings: Bindings => "bindings", correlation_id: Correlation => "correlationId"
});
node!(metadata_access Tag {
    reference: String => "$ref", name: String => "name", description: String => "description",
    external_docs: ExternalDocumentation => "externalDocs"
});
node!(metadata_access Parameter {
    reference: String => "$ref", schema: SchemaValue => "schema"
});
node!(metadata_access Correlation {
    reference: String => "$ref", location: String => "location"
});
node!(metadata_access Bindings {
    reference: String => "$ref", ws: Ws => "ws"
});
node!(metadata_access Ws {
    reference: String => "$ref", headers: SchemaValue => "headers", query: SchemaValue => "query"
});

macro_rules! node_views {
    ($($variant:ident => $ty:ty),* $(,)?) => {
        /// Borrowed structure. Raw values are unknown vocabulary or deferred invalid fields.
        #[derive(Serialize)]
        #[serde(untagged)]
        pub enum Node<'a> { $($variant(&'a $ty),)* }
        pub enum NodeMut<'a> { $($variant(&'a mut $ty),)* }
        pub trait StructuralNode {
            fn node(&self) -> Node<'_>;
            fn node_mut(&mut self) -> NodeMut<'_>;
        }
        $(impl StructuralNode for $ty {
            fn node(&self) -> Node<'_> { Node::$variant(self) }
            fn node_mut(&mut self) -> NodeMut<'_> { NodeMut::$variant(self) }
        })*
        impl NodeMut<'_> {
            pub fn as_ref(&self) -> Node<'_> {
                match self { $(Self::$variant(value) => Node::$variant(value),)* }
            }
        }
    };
}
node_views!(Metadata => Metadata, Tag => Tag, Parameter => Parameter,
    Correlation => Correlation, Bindings => Bindings, Ws => Ws,
    Schema => SchemaValue, Documentation => ExternalDocumentation, Raw => Value);

macro_rules! metadata_access {
    ($($variant:ident),* $(,)?) => {
        impl Node<'_> {
            pub fn keyword_names(&self) -> Vec<&str> {
                match self {
                    $(Self::$variant(value) => value.keyword_names(),)*
                    Self::Schema(SchemaValue::Object(value)) => value.keyword_names(),
                    Self::Raw(Value::Object(value)) => value.keys().map(String::as_str).collect(),
                    _ => Vec::new(),
                }
            }
            pub fn serialize_keyword<S: Serializer>(&self, key: &str, serializer: S) -> Result<Option<S::Ok>, S::Error> {
                match self {
                    $(Self::$variant(value) => value.serialize_keyword(key, serializer),)*
                    Self::Schema(SchemaValue::Object(value)) => value.serialize_keyword(key, serializer),
                    Self::Raw(value) => value.get(key).map(|value| value.serialize(serializer)).transpose(),
                    _ => Ok(None),
                }
            }
        }
    };
}
metadata_access!(
    Metadata,
    Tag,
    Parameter,
    Correlation,
    Bindings,
    Ws,
    Documentation
);

macro_rules! node_children {
    ($module:ident, $view:ident, [$($mutability:tt)?], $node:ident, $reference:ident) => {
        mod $module {
            use super::*;
            fn child<T: StructuralNode, E>(field: & $($mutability)? Field<T>, key: &str,
                visit: &mut impl FnMut(&str, $view<'_>) -> Result<(), E>) -> Result<(), E> {
                match field {
                    Field::Missing => Ok(()),
                    Field::Invalid(value) => visit(key, $view::Raw(value)),
                    Field::Value(value) => visit(key, value.$node()),
                }
            }
            impl $view<'_> {
                pub fn $reference(& $($mutability)? self) -> Option<& $($mutability)? Field<String>> {
                    match self {
                        Self::Tag(value) => Some(& $($mutability)? value.reference),
                        Self::Parameter(value) => Some(& $($mutability)? value.reference),
                        Self::Correlation(value) => Some(& $($mutability)? value.reference),
                        Self::Bindings(value) => Some(& $($mutability)? value.reference),
                        Self::Ws(value) => Some(& $($mutability)? value.reference),
                        Self::Documentation(value) => Some(& $($mutability)? value.reference),
                        _ => None,
                    }
                }
                /// Visit immediate structural children in lexical order. Schema contents
                /// use the schema visitor; callers own recursion and reference policy.
                pub fn try_for_each_child<E>(& $($mutability)? self,
                    mut visit: impl FnMut(&str, $view<'_>) -> Result<(), E>) -> Result<(), E> {
                    match self {
                        Self::Metadata(value) => {
                            child(& $($mutability)? value.bindings, "bindings", &mut visit)?;
                            child(& $($mutability)? value.correlation_id, "correlationId", &mut visit)?;
                            child(& $($mutability)? value.external_docs, "externalDocs", &mut visit)?;
                            match & $($mutability)? value.tags {
                                Field::Missing => {},
                                Field::Invalid(value) => visit("tags", $view::Raw(value))?,
                                Field::Value(values) => for value in values { child(value, "tags", &mut visit)?; },
                            }
                        }
                        Self::Tag(value) => child(& $($mutability)? value.external_docs, "externalDocs", &mut visit)?,
                        Self::Parameter(value) => child(& $($mutability)? value.schema, "schema", &mut visit)?,
                        Self::Bindings(value) => child(& $($mutability)? value.ws, "ws", &mut visit)?,
                        Self::Ws(value) => {
                            child(& $($mutability)? value.headers, "headers", &mut visit)?;
                            child(& $($mutability)? value.query, "query", &mut visit)?;
                        }
                        _ => {},
                    }
                    Ok(())
                }
            }
        }
    };
}
node_children!(borrowed_children, Node, [], node, reference);
node_children!(editable_children, NodeMut, [mut], node_mut, reference_mut);

/// Look up structure using already decoded tokens, without following references.
pub trait NodeLookup {
    fn node_at_tokens<T: AsRef<str>>(&self, tokens: &[T]) -> Option<Node<'_>>;
}

impl<T: NodeLookup> NodeLookup for Field<T> {
    fn node_at_tokens<S: AsRef<str>>(&self, tokens: &[S]) -> Option<Node<'_>> {
        match self {
            Field::Missing => None,
            Field::Value(value) => value.node_at_tokens(tokens),
            Field::Invalid(value) => raw_at_tokens(value, tokens).map(Node::Raw),
        }
    }
}
impl<T: NodeLookup> NodeLookup for List<T> {
    fn node_at_tokens<S: AsRef<str>>(&self, tokens: &[S]) -> Option<Node<'_>> {
        let (index, tail) = tokens.split_first()?;
        self.get(index.as_ref().parse::<usize>().ok()?)?
            .node_at_tokens(tail)
    }
}
impl<T: NodeLookup> NodeLookup for Fields<T> {
    fn node_at_tokens<S: AsRef<str>>(&self, tokens: &[S]) -> Option<Node<'_>> {
        let (key, tail) = tokens.split_first()?;
        self.get(key.as_ref())?.node_at_tokens(tail)
    }
}
impl NodeLookup for SchemaValue {
    fn node_at_tokens<T: AsRef<str>>(&self, tokens: &[T]) -> Option<Node<'_>> {
        if let Some(schema) = self.schema_at_tokens(tokens) {
            return Some(Node::Schema(schema));
        }
        for (index, token) in tokens.iter().enumerate() {
            if token.as_ref() != "externalDocs" {
                continue;
            }
            if let Some(docs) = self
                .schema_at_tokens(&tokens[..index])
                .and_then(SchemaValue::as_object)
                .and_then(|schema| schema.external_docs.as_ref())
            {
                return docs.node_at_tokens(&tokens[index + 1..]);
            }
        }
        None
    }
}

macro_rules! lookup {
    ($name:ident, $variant:ident { $($key:literal => $field:ident),* $(,)? }) => {
        impl NodeLookup for $name {
            fn node_at_tokens<T: AsRef<str>>(&self, tokens: &[T]) -> Option<Node<'_>> {
                let Some((key, tail)) = tokens.split_first() else {
                    return Some(Node::$variant(self));
                };
                match key.as_ref() {
                    $($key => self.$field.node_at_tokens(tail),)*
                    key => raw_at_tokens(self.extras.get(key)?, tail).map(Node::Raw),
                }
            }
        }
    };
}
lookup!(Metadata, Metadata { "tags" => tags, "externalDocs" => external_docs,
    "bindings" => bindings, "correlationId" => correlation_id });
lookup!(Tag, Tag { "externalDocs" => external_docs });
lookup!(Parameter, Parameter { "schema" => schema });
lookup!(Correlation, Correlation {});
lookup!(Bindings, Bindings { "ws" => ws });
lookup!(Ws, Ws { "headers" => headers, "query" => query });
lookup!(ExternalDocumentation, Documentation {});

fn raw_at_tokens<'a, T: AsRef<str>>(value: &'a Value, tokens: &[T]) -> Option<&'a Value> {
    tokens.iter().try_fold(value, |value, token| match value {
        Value::Array(values) => values.get(token.as_ref().parse::<usize>().ok()?),
        _ => value.get(token.as_ref()),
    })
}
