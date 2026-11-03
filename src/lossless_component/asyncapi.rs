//! Lossless AsyncAPI structural fields for selective document consumers.
//! Decoding establishes shape, not protocol validity or reference permission.
use super::*;

node!(metadata_collections Components {
    schemas: Fields<SchemaValue> => "schemas", servers: Fields<Server> => "servers",
    channels: Fields<Channel> => "channels", operations: Fields<Operation> => "operations",
    messages: Fields<Message> => "messages", security_schemes: Fields<SecurityScheme> => "securitySchemes",
    server_variables: Fields<ServerVariable> => "serverVariables",
    parameters: Fields<Parameter> => "parameters", correlation_ids: Fields<Correlation> => "correlationIds",
    replies: Fields<OperationReply> => "replies", reply_addresses: Fields<OperationReplyAddress> => "replyAddresses",
    message_traits: Fields<MessageTrait> => "messageTraits", operation_traits: Fields<OperationTrait> => "operationTraits",
    server_bindings: Fields<Bindings> => "serverBindings",
    channel_bindings: Fields<Bindings> => "channelBindings", operation_bindings: Fields<Bindings> => "operationBindings",
    message_bindings: Fields<Bindings> => "messageBindings", tags: Fields<Tag> => "tags",
    external_docs: Fields<ExternalDocumentation> => "externalDocs"
});

node!(metadata_selected Metadata {
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

// Root and structural objects. `info` and other undeclared vocabulary stay in extras.
node!(metadata_access Document {
    asyncapi: String => "asyncapi", id: String => "id", default_content_type: String => "defaultContentType",
    servers: Fields<Server> => "servers", channels: Fields<Channel> => "channels",
    operations: Fields<Operation> => "operations", components: Components => "components"
});
// A Reference Object where nothing else is admitted, such as `channel` and `messages`.
node!(metadata_access Reference { reference: String => "$ref" });
node!(metadata_access Server {
    reference: String => "$ref", host: String => "host", protocol: String => "protocol",
    protocol_version: String => "protocolVersion", pathname: String => "pathname",
    description: String => "description", title: String => "title", summary: String => "summary",
    variables: Fields<ServerVariable> => "variables", security: List<SecurityScheme> => "security",
    tags: List<Tag> => "tags", external_docs: ExternalDocumentation => "externalDocs", bindings: Bindings => "bindings"
});
node!(metadata_access ServerVariable {
    reference: String => "$ref", values: List<String> => "enum", default: String => "default",
    description: String => "description", examples: List<String> => "examples"
});
// `address: null` (an unknown or dynamic address) stays distinct from a missing address.
node!(metadata_access Channel {
    reference: String => "$ref", address: String => "address", messages: Fields<Message> => "messages",
    title: String => "title", summary: String => "summary", description: String => "description",
    servers: List<Reference> => "servers", parameters: Fields<Parameter> => "parameters",
    tags: List<Tag> => "tags", external_docs: ExternalDocumentation => "externalDocs", bindings: Bindings => "bindings"
});
node!(metadata_access Operation {
    reference: String => "$ref", action: String => "action", channel: Reference => "channel",
    title: String => "title", summary: String => "summary", description: String => "description",
    security: List<SecurityScheme> => "security", tags: List<Tag> => "tags",
    external_docs: ExternalDocumentation => "externalDocs", bindings: Bindings => "bindings",
    traits: List<OperationTrait> => "traits", messages: List<Reference> => "messages", reply: OperationReply => "reply"
});
node!(metadata_access OperationTrait {
    reference: String => "$ref", title: String => "title", summary: String => "summary",
    description: String => "description", security: List<SecurityScheme> => "security", tags: List<Tag> => "tags",
    external_docs: ExternalDocumentation => "externalDocs", bindings: Bindings => "bindings"
});
node!(metadata_access OperationReply {
    reference: String => "$ref", address: OperationReplyAddress => "address", channel: Reference => "channel",
    messages: List<Reference> => "messages"
});
node!(metadata_access OperationReplyAddress {
    reference: String => "$ref", description: String => "description", location: String => "location"
});
// `headers` and `payload` are schemas, including `schemaFormat` wrappers. Examples are literal.
node!(metadata_access Message {
    reference: String => "$ref", headers: SchemaValue => "headers", payload: SchemaValue => "payload",
    correlation_id: Correlation => "correlationId", content_type: String => "contentType", name: String => "name",
    title: String => "title", summary: String => "summary", description: String => "description",
    tags: List<Tag> => "tags", external_docs: ExternalDocumentation => "externalDocs", bindings: Bindings => "bindings",
    examples: List<Value> => "examples", traits: List<MessageTrait> => "traits"
});
node!(metadata_access MessageTrait {
    reference: String => "$ref", headers: SchemaValue => "headers", correlation_id: Correlation => "correlationId",
    content_type: String => "contentType", name: String => "name", title: String => "title",
    summary: String => "summary", description: String => "description", tags: List<Tag> => "tags",
    external_docs: ExternalDocumentation => "externalDocs", bindings: Bindings => "bindings",
    examples: List<Value> => "examples"
});
node!(metadata_access SecurityScheme {
    reference: String => "$ref", kind: String => "type", description: String => "description",
    name: String => "name", location: String => "in", scheme: String => "scheme", bearer_format: String => "bearerFormat",
    flows: OauthFlows => "flows", open_id_connect_url: String => "openIdConnectUrl", scopes: List<String> => "scopes"
});
node!(metadata_access OauthFlows {
    implicit: OauthFlow => "implicit", password: OauthFlow => "password",
    client_credentials: OauthFlow => "clientCredentials", authorization_code: OauthFlow => "authorizationCode"
});
node!(metadata_access OauthFlow {
    authorization_url: String => "authorizationUrl", token_url: String => "tokenUrl",
    refresh_url: String => "refreshUrl", available_scopes: Fields<String> => "availableScopes"
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
    Document => Document, Components => Components, Reference => Reference, Server => Server,
    ServerVariable => ServerVariable, Channel => Channel, Operation => Operation,
    OperationTrait => OperationTrait, OperationReply => OperationReply,
    OperationReplyAddress => OperationReplyAddress, Message => Message, MessageTrait => MessageTrait,
    SecurityScheme => SecurityScheme, OauthFlows => OauthFlows, OauthFlow => OauthFlow,
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
    Document,
    Components,
    Reference,
    Server,
    ServerVariable,
    Channel,
    Operation,
    OperationTrait,
    OperationReply,
    OperationReplyAddress,
    Message,
    MessageTrait,
    SecurityScheme,
    OauthFlows,
    OauthFlow,
    Documentation
);

macro_rules! node_children {
    ($module:ident, $view:ident, [$($mutability:tt)?], $node:ident, $reference:ident, $iter:ident) => {
        mod $module {
            use super::*;
            use crate::json_pointer::Step;
            fn at<T: StructuralNode, E>(field: & $($mutability)? Field<T>, step: Step<'_>,
                visit: &mut impl FnMut(Step<'_>, $view<'_>) -> Result<(), E>) -> Result<(), E> {
                match field {
                    Field::Missing => Ok(()),
                    Field::Invalid(value) => visit(step, $view::Raw(value)),
                    Field::Value(value) => visit(step, value.$node()),
                }
            }
            fn child<T: StructuralNode, E>(field: & $($mutability)? Field<T>, key: &str,
                visit: &mut impl FnMut(Step<'_>, $view<'_>) -> Result<(), E>) -> Result<(), E> {
                at(field, Step::Token(key), visit)
            }
            // A malformed collection is one raw child under its declaring field name.
            fn children<T: StructuralNode, E>(field: & $($mutability)? Field<Fields<T>>, key: &str,
                visit: &mut impl FnMut(Step<'_>, $view<'_>) -> Result<(), E>) -> Result<(), E> {
                match field {
                    Field::Missing => Ok(()),
                    Field::Invalid(value) => visit(Step::Token(key), $view::Raw(value)),
                    Field::Value(values) => values.$iter()
                        .try_for_each(|(name, value)| at(value, Step::Entry { field: key, key: name }, visit)),
                }
            }
            fn elements<T: StructuralNode, E>(field: & $($mutability)? Field<List<T>>, key: &str,
                visit: &mut impl FnMut(Step<'_>, $view<'_>) -> Result<(), E>) -> Result<(), E> {
                match field {
                    Field::Missing => Ok(()),
                    Field::Invalid(value) => visit(Step::Token(key), $view::Raw(value)),
                    Field::Value(values) => values.$iter().enumerate()
                        .try_for_each(|(index, value)| at(value, Step::Element { field: key, index }, visit)),
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
                        Self::Reference(value) => Some(& $($mutability)? value.reference),
                        Self::Server(value) => Some(& $($mutability)? value.reference),
                        Self::ServerVariable(value) => Some(& $($mutability)? value.reference),
                        Self::Channel(value) => Some(& $($mutability)? value.reference),
                        Self::Operation(value) => Some(& $($mutability)? value.reference),
                        Self::OperationTrait(value) => Some(& $($mutability)? value.reference),
                        Self::OperationReply(value) => Some(& $($mutability)? value.reference),
                        Self::OperationReplyAddress(value) => Some(& $($mutability)? value.reference),
                        Self::Message(value) => Some(& $($mutability)? value.reference),
                        Self::MessageTrait(value) => Some(& $($mutability)? value.reference),
                        Self::SecurityScheme(value) => Some(& $($mutability)? value.reference),
                        _ => None,
                    }
                }
                /// Visit immediate structural children in lexical order. Schema contents
                /// use the schema visitor; callers own recursion and reference policy.
                /// Map entries and list elements are visited under their declaring field
                /// name; use `try_for_each_child_at` for their key or index.
                pub fn try_for_each_child<E>(& $($mutability)? self,
                    mut visit: impl FnMut(&str, $view<'_>) -> Result<(), E>) -> Result<(), E> {
                    self.try_for_each_child_at(|step, child| visit(step.name(), child))
                }

                /// Visit the children of `try_for_each_child`, in the same order, with
                /// the [`Step`] that locates each one under this node: `/field` for a
                /// field, `/field/key` for a map entry and `/field/index` for a list
                /// element. A malformed collection is one `Raw` child at `/field`.
                /// [`Step::name`] is the name `try_for_each_child` reports.
                pub fn try_for_each_child_at<E>(& $($mutability)? self,
                    mut visit: impl FnMut(Step<'_>, $view<'_>) -> Result<(), E>) -> Result<(), E> {
                    match self {
                        Self::Metadata(value) => {
                            child(& $($mutability)? value.bindings, "bindings", &mut visit)?;
                            child(& $($mutability)? value.correlation_id, "correlationId", &mut visit)?;
                            child(& $($mutability)? value.external_docs, "externalDocs", &mut visit)?;
                            elements(& $($mutability)? value.tags, "tags", &mut visit)?;
                        }
                        Self::Tag(value) => child(& $($mutability)? value.external_docs, "externalDocs", &mut visit)?,
                        Self::Parameter(value) => child(& $($mutability)? value.schema, "schema", &mut visit)?,
                        Self::Bindings(value) => child(& $($mutability)? value.ws, "ws", &mut visit)?,
                        Self::Ws(value) => {
                            child(& $($mutability)? value.headers, "headers", &mut visit)?;
                            child(& $($mutability)? value.query, "query", &mut visit)?;
                        }
                        Self::Document(value) => {
                            children(& $($mutability)? value.channels, "channels", &mut visit)?;
                            child(& $($mutability)? value.components, "components", &mut visit)?;
                            children(& $($mutability)? value.operations, "operations", &mut visit)?;
                            children(& $($mutability)? value.servers, "servers", &mut visit)?;
                        }
                        Self::Components(value) => {
                            children(& $($mutability)? value.channel_bindings, "channelBindings", &mut visit)?;
                            children(& $($mutability)? value.channels, "channels", &mut visit)?;
                            children(& $($mutability)? value.correlation_ids, "correlationIds", &mut visit)?;
                            children(& $($mutability)? value.external_docs, "externalDocs", &mut visit)?;
                            children(& $($mutability)? value.message_bindings, "messageBindings", &mut visit)?;
                            children(& $($mutability)? value.message_traits, "messageTraits", &mut visit)?;
                            children(& $($mutability)? value.messages, "messages", &mut visit)?;
                            children(& $($mutability)? value.operation_bindings, "operationBindings", &mut visit)?;
                            children(& $($mutability)? value.operation_traits, "operationTraits", &mut visit)?;
                            children(& $($mutability)? value.operations, "operations", &mut visit)?;
                            children(& $($mutability)? value.parameters, "parameters", &mut visit)?;
                            children(& $($mutability)? value.replies, "replies", &mut visit)?;
                            children(& $($mutability)? value.reply_addresses, "replyAddresses", &mut visit)?;
                            children(& $($mutability)? value.schemas, "schemas", &mut visit)?;
                            children(& $($mutability)? value.security_schemes, "securitySchemes", &mut visit)?;
                            children(& $($mutability)? value.server_bindings, "serverBindings", &mut visit)?;
                            children(& $($mutability)? value.server_variables, "serverVariables", &mut visit)?;
                            children(& $($mutability)? value.servers, "servers", &mut visit)?;
                            children(& $($mutability)? value.tags, "tags", &mut visit)?;
                        }
                        Self::Server(value) => {
                            child(& $($mutability)? value.bindings, "bindings", &mut visit)?;
                            child(& $($mutability)? value.external_docs, "externalDocs", &mut visit)?;
                            elements(& $($mutability)? value.security, "security", &mut visit)?;
                            elements(& $($mutability)? value.tags, "tags", &mut visit)?;
                            children(& $($mutability)? value.variables, "variables", &mut visit)?;
                        }
                        Self::Channel(value) => {
                            child(& $($mutability)? value.bindings, "bindings", &mut visit)?;
                            child(& $($mutability)? value.external_docs, "externalDocs", &mut visit)?;
                            children(& $($mutability)? value.messages, "messages", &mut visit)?;
                            children(& $($mutability)? value.parameters, "parameters", &mut visit)?;
                            elements(& $($mutability)? value.servers, "servers", &mut visit)?;
                            elements(& $($mutability)? value.tags, "tags", &mut visit)?;
                        }
                        Self::Operation(value) => {
                            child(& $($mutability)? value.bindings, "bindings", &mut visit)?;
                            child(& $($mutability)? value.channel, "channel", &mut visit)?;
                            child(& $($mutability)? value.external_docs, "externalDocs", &mut visit)?;
                            elements(& $($mutability)? value.messages, "messages", &mut visit)?;
                            child(& $($mutability)? value.reply, "reply", &mut visit)?;
                            elements(& $($mutability)? value.security, "security", &mut visit)?;
                            elements(& $($mutability)? value.tags, "tags", &mut visit)?;
                            elements(& $($mutability)? value.traits, "traits", &mut visit)?;
                        }
                        Self::OperationTrait(value) => {
                            child(& $($mutability)? value.bindings, "bindings", &mut visit)?;
                            child(& $($mutability)? value.external_docs, "externalDocs", &mut visit)?;
                            elements(& $($mutability)? value.security, "security", &mut visit)?;
                            elements(& $($mutability)? value.tags, "tags", &mut visit)?;
                        }
                        Self::OperationReply(value) => {
                            child(& $($mutability)? value.address, "address", &mut visit)?;
                            child(& $($mutability)? value.channel, "channel", &mut visit)?;
                            elements(& $($mutability)? value.messages, "messages", &mut visit)?;
                        }
                        Self::Message(value) => {
                            child(& $($mutability)? value.bindings, "bindings", &mut visit)?;
                            child(& $($mutability)? value.correlation_id, "correlationId", &mut visit)?;
                            child(& $($mutability)? value.external_docs, "externalDocs", &mut visit)?;
                            child(& $($mutability)? value.headers, "headers", &mut visit)?;
                            child(& $($mutability)? value.payload, "payload", &mut visit)?;
                            elements(& $($mutability)? value.tags, "tags", &mut visit)?;
                            elements(& $($mutability)? value.traits, "traits", &mut visit)?;
                        }
                        Self::MessageTrait(value) => {
                            child(& $($mutability)? value.bindings, "bindings", &mut visit)?;
                            child(& $($mutability)? value.correlation_id, "correlationId", &mut visit)?;
                            child(& $($mutability)? value.external_docs, "externalDocs", &mut visit)?;
                            child(& $($mutability)? value.headers, "headers", &mut visit)?;
                            elements(& $($mutability)? value.tags, "tags", &mut visit)?;
                        }
                        Self::SecurityScheme(value) => {
                            child(& $($mutability)? value.flows, "flows", &mut visit)?;
                        }
                        Self::OauthFlows(value) => {
                            child(& $($mutability)? value.authorization_code, "authorizationCode", &mut visit)?;
                            child(& $($mutability)? value.client_credentials, "clientCredentials", &mut visit)?;
                            child(& $($mutability)? value.implicit, "implicit", &mut visit)?;
                            child(& $($mutability)? value.password, "password", &mut visit)?;
                        }
                        _ => {},
                    }
                    Ok(())
                }
            }
        }
    };
}
node_children!(borrowed_children, Node, [], node, reference, iter);
node_children!(editable_children, NodeMut, [mut], node_mut, reference_mut, iter_mut);

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
lookup!(Document, Document { "channels" => channels, "components" => components,
    "operations" => operations, "servers" => servers });
lookup!(Components, Components {
    "schemas" => schemas, "servers" => servers, "channels" => channels, "operations" => operations,
    "messages" => messages, "securitySchemes" => security_schemes, "serverVariables" => server_variables,
    "parameters" => parameters, "correlationIds" => correlation_ids, "replies" => replies,
    "replyAddresses" => reply_addresses, "messageTraits" => message_traits, "operationTraits" => operation_traits,
    "serverBindings" => server_bindings, "channelBindings" => channel_bindings,
    "operationBindings" => operation_bindings, "messageBindings" => message_bindings, "tags" => tags,
    "externalDocs" => external_docs });
lookup!(Reference, Reference {});
lookup!(Server, Server { "variables" => variables, "security" => security, "tags" => tags,
    "externalDocs" => external_docs, "bindings" => bindings });
lookup!(ServerVariable, ServerVariable {});
lookup!(Channel, Channel { "messages" => messages, "servers" => servers, "parameters" => parameters,
    "tags" => tags, "externalDocs" => external_docs, "bindings" => bindings });
lookup!(Operation, Operation { "channel" => channel, "security" => security, "tags" => tags,
    "externalDocs" => external_docs, "bindings" => bindings, "traits" => traits, "messages" => messages,
    "reply" => reply });
lookup!(OperationTrait, OperationTrait { "security" => security, "tags" => tags,
    "externalDocs" => external_docs, "bindings" => bindings });
lookup!(OperationReply, OperationReply { "address" => address, "channel" => channel, "messages" => messages });
lookup!(OperationReplyAddress, OperationReplyAddress {});
lookup!(Message, Message { "headers" => headers, "payload" => payload, "correlationId" => correlation_id,
    "tags" => tags, "externalDocs" => external_docs, "bindings" => bindings, "traits" => traits });
lookup!(MessageTrait, MessageTrait { "headers" => headers, "correlationId" => correlation_id,
    "tags" => tags, "externalDocs" => external_docs, "bindings" => bindings });
lookup!(SecurityScheme, SecurityScheme { "flows" => flows });
lookup!(OauthFlows, OauthFlows { "implicit" => implicit, "password" => password,
    "clientCredentials" => client_credentials, "authorizationCode" => authorization_code });
lookup!(OauthFlow, OauthFlow {});

fn raw_at_tokens<'a, T: AsRef<str>>(value: &'a Value, tokens: &[T]) -> Option<&'a Value> {
    tokens.iter().try_fold(value, |value, token| match value {
        Value::Array(values) => values.get(token.as_ref().parse::<usize>().ok()?),
        _ => value.get(token.as_ref()),
    })
}
