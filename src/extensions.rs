//! Extension methods and convenience constructors for OpenAPI types.
//!
//! This module provides convenient constructors, builder methods, and helper functions
//! for all the main OpenAPI types to make them easier to work with programmatically.

use std::collections::BTreeMap;

use crate::types::*;

// Convenience constructors for main types
impl OpenAPIV3 {
    pub fn new(info: Info) -> Self {
        Self {
            openapi: "3.0.0".to_string(),
            info,
            servers: None,
            paths: BTreeMap::new(),
            components: None,
            security: None,
            tags: None,
            external_docs: None,
            extras: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.info.description = Some(description.into());
        self
    }

    pub fn with_paths(mut self, paths: BTreeMap<String, PathItem>) -> Self {
        self.paths = paths;
        self
    }

    pub fn with_components(mut self, components: Components) -> Self {
        self.components = Some(components);
        self
    }

    pub fn with_servers(mut self, servers: Vec<Server>) -> Self {
        self.servers = Some(servers);
        self
    }
}

impl Info {
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            terms_of_service: None,
            contact: None,
            license: None,
            version: version.into(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_contact(mut self, contact: Contact) -> Self {
        self.contact = Some(contact);
        self
    }

    pub fn with_license(mut self, license: License) -> Self {
        self.license = Some(license);
        self
    }
}

impl Contact {
    pub fn new() -> Self {
        Self {
            name: None,
            url: None,
            email: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }
}

impl Default for Contact {
    fn default() -> Self {
        Self::new()
    }
}

impl License {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: None,
        }
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}

impl Server {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            description: None,
            variables: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_variables(mut self, variables: BTreeMap<String, ServerVariable>) -> Self {
        self.variables = Some(variables);
        self
    }
}

impl ServerVariable {
    pub fn new(default: impl Into<String>) -> Self {
        Self {
            _enum: None,
            default: default.into(),
            description: None,
        }
    }

    pub fn with_enum(mut self, enum_values: Vec<String>) -> Self {
        self._enum = Some(enum_values);
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl PathItem {
    pub fn new() -> Self {
        Self {
            _ref: None,
            summary: None,
            description: None,
            get: None,
            put: None,
            post: None,
            delete: None,
            options: None,
            head: None,
            patch: None,
            trace: None,
            servers: None,
            parameters: None,
        }
    }

    pub fn with_get(mut self, operation: Operation) -> Self {
        self.get = Some(operation);
        self
    }

    pub fn with_post(mut self, operation: Operation) -> Self {
        self.post = Some(operation);
        self
    }

    pub fn with_put(mut self, operation: Operation) -> Self {
        self.put = Some(operation);
        self
    }

    pub fn with_delete(mut self, operation: Operation) -> Self {
        self.delete = Some(operation);
        self
    }

    pub fn with_patch(mut self, operation: Operation) -> Self {
        self.patch = Some(operation);
        self
    }
}

impl Default for PathItem {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation {
    pub fn new(responses: Responses) -> Self {
        Self {
            tags: None,
            summary: None,
            description: None,
            external_docs: None,
            operation_id: None,
            parameters: None,
            request_body: None,
            responses,
            callbacks: None,
            deprecated: None,
            security: None,
            servers: None,
        }
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_operation_id(mut self, operation_id: impl Into<String>) -> Self {
        self.operation_id = Some(operation_id.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_parameters(mut self, parameters: Vec<Referenceable<Parameter>>) -> Self {
        self.parameters = Some(parameters);
        self
    }

    pub fn with_request_body(mut self, request_body: Referenceable<RequestBody>) -> Self {
        self.request_body = Some(request_body);
        self
    }
}

impl Parameter {
    pub fn new(name: impl Into<String>, location: ParameterIn) -> Self {
        Self {
            name: name.into(),
            _in: location,
            description: None,
            required: None,
            deprecated: None,
            allow_empty_value: None,
            style: None,
            explode: None,
            allow_reserved: None,
            schema: None,
            example: None,
            examples: None,
            content: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    pub fn with_schema(mut self, schema: Referenceable<Schema>) -> Self {
        self.schema = Some(schema);
        self
    }
}

impl RequestBody {
    pub fn new(content: BTreeMap<String, MediaType>) -> Self {
        Self {
            description: None,
            required: None,
            content,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }
}

impl Response {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            headers: None,
            content: None,
            links: None,
        }
    }

    pub fn with_content(mut self, content: BTreeMap<String, MediaType>) -> Self {
        self.content = Some(content);
        self
    }

    pub fn with_headers(mut self, headers: BTreeMap<String, Referenceable<Header>>) -> Self {
        self.headers = Some(headers);
        self
    }
}

impl Responses {
    pub fn new() -> Self {
        Self {
            default: None,
            data: BTreeMap::new(),
        }
    }

    pub fn with_status(mut self, status: impl Into<String>, response: Referenceable<Response>) -> Self {
        self.data.insert(status.into(), response);
        self
    }

    pub fn with_default(mut self, response: Referenceable<Response>) -> Self {
        self.default = Some(response);
        self
    }
}

impl Default for Responses {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaType {
    pub fn new() -> Self {
        Self {
            schema: None,
            example: None,
            examples: None,
            encoding: None,
        }
    }

    pub fn with_schema(mut self, schema: Referenceable<Schema>) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn with_example(mut self, example: Any) -> Self {
        self.example = Some(example);
        self
    }
}

impl Default for MediaType {
    fn default() -> Self {
        Self::new()
    }
}

impl Schema {
    pub fn new() -> Self {
        Self {
            _type: None,
            format: None,
            nullable: None,
            description: None,
            extras: BTreeMap::new(),
        }
    }

    pub fn with_type(mut self, schema_type: impl Into<String>) -> Self {
        self._type = Some(schema_type.into());
        self
    }

    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn string() -> Self {
        Self::new().with_type("string")
    }

    pub fn integer() -> Self {
        Self::new().with_type("integer")
    }

    pub fn number() -> Self {
        Self::new().with_type("number")
    }

    pub fn boolean() -> Self {
        Self::new().with_type("boolean")
    }

    pub fn array() -> Self {
        Self::new().with_type("array")
    }

    pub fn object() -> Self {
        Self::new().with_type("object")
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

impl Components {
    pub fn new() -> Self {
        Self {
            schemas: None,
            responses: None,
            parameters: None,
            examples: None,
            request_bodies: None,
            headers: None,
            security_schemes: None,
            links: None,
            callbacks: None,
        }
    }

    pub fn with_schemas(mut self, schemas: BTreeMap<String, Referenceable<Schema>>) -> Self {
        self.schemas = Some(schemas);
        self
    }

    pub fn with_responses(mut self, responses: BTreeMap<String, Referenceable<Response>>) -> Self {
        self.responses = Some(responses);
        self
    }

    pub fn with_parameters(mut self, parameters: BTreeMap<String, Referenceable<Parameter>>) -> Self {
        self.parameters = Some(parameters);
        self
    }
}

impl Default for Components {
    fn default() -> Self {
        Self::new()
    }
}

impl Tag {
    pub fn new(name: impl Into<String>, description: Option<impl Into<String>>) -> Tag {
        Self {
            name: name.into(),
            description: description.map(|d| d.into()),
            external_docs: None,
        }
    }

    pub fn simple(name: impl Into<String>) -> Tag {
        Self {
            name: name.into(),
            description: None,
            external_docs: None,
        }
    }

    pub fn with_description(name: impl Into<String>, description: impl Into<String>) -> Tag {
        Self {
            name: name.into(),
            description: Some(description.into()),
            external_docs: None,
        }
    }
}

// Common helper functions and convenience methods
impl Referenceable<Schema> {
    /// Create a reference to a schema component
    pub fn schema_ref(name: impl Into<String>) -> Self {
        Self::component_ref("schemas", name)
    }

    /// Create inline string schema
    pub fn string_schema() -> Self {
        Self::data(Schema::string())
    }

    /// Create inline integer schema
    pub fn integer_schema() -> Self {
        Self::data(Schema::integer())
    }

    /// Create inline number schema
    pub fn number_schema() -> Self {
        Self::data(Schema::number())
    }

    /// Create inline boolean schema
    pub fn boolean_schema() -> Self {
        Self::data(Schema::boolean())
    }

    /// Create inline array schema
    pub fn array_schema() -> Self {
        Self::data(Schema::array())
    }

    /// Create inline object schema
    pub fn object_schema() -> Self {
        Self::data(Schema::object())
    }
}

impl Referenceable<Response> {
    /// Create a reference to a response component
    pub fn response_ref(name: impl Into<String>) -> Self {
        Self::component_ref("responses", name)
    }

    /// Create a simple success response
    pub fn ok(description: impl Into<String>) -> Self {
        Self::data(Response::new(description))
    }

    /// Create a simple error response
    pub fn error(description: impl Into<String>) -> Self {
        Self::data(Response::new(description))
    }

    /// Add content to response if it contains data
    pub fn with_content(self, content: BTreeMap<String, MediaType>) -> Self {
        match self {
            Self::Data(response) => Self::data(response.with_content(content)),
            Self::Reference(r) => Self::Reference(r),
        }
    }

    /// Add headers to response if it contains data
    pub fn with_headers(self, headers: BTreeMap<String, Referenceable<Header>>) -> Self {
        match self {
            Self::Data(response) => Self::data(response.with_headers(headers)),
            Self::Reference(r) => Self::Reference(r),
        }
    }
}

impl Referenceable<Parameter> {
    /// Create a reference to a parameter component
    pub fn parameter_ref(name: impl Into<String>) -> Self {
        Self::component_ref("parameters", name)
    }

    /// Create a query parameter
    pub fn query_param(name: impl Into<String>) -> Self {
        Self::data(Parameter::new(name, ParameterIn::Query))
    }

    /// Create a path parameter
    pub fn path_param(name: impl Into<String>) -> Self {
        Self::data(Parameter::new(name, ParameterIn::Path).with_required(true))
    }

    /// Create a header parameter
    pub fn header_param(name: impl Into<String>) -> Self {
        Self::data(Parameter::new(name, ParameterIn::Header))
    }

    /// Add schema to parameter if it contains data
    pub fn with_schema(self, schema: Referenceable<Schema>) -> Self {
        match self {
            Self::Data(param) => Self::data(param.with_schema(schema)),
            Self::Reference(r) => Self::Reference(r),
        }
    }

    /// Add description to parameter if it contains data
    pub fn with_description(self, description: impl Into<String>) -> Self {
        match self {
            Self::Data(param) => Self::data(param.with_description(description)),
            Self::Reference(r) => Self::Reference(r),
        }
    }

    /// Mark parameter as required if it contains data
    pub fn with_required(self, required: bool) -> Self {
        match self {
            Self::Data(param) => Self::data(param.with_required(required)),
            Self::Reference(r) => Self::Reference(r),
        }
    }
}

impl Referenceable<RequestBody> {
    /// Create a reference to a request body component
    pub fn request_body_ref(name: impl Into<String>) -> Self {
        Self::component_ref("requestBodies", name)
    }

    /// Create a JSON request body
    pub fn json_body(schema: Referenceable<Schema>) -> Self {
        let mut content = BTreeMap::new();
        content.insert(
            "application/json".to_string(),
            MediaType::new().with_schema(schema),
        );
        Self::data(RequestBody::new(content))
    }
}

// Additional convenience methods for PathItem
impl PathItem {
    /// Add multiple HTTP methods at once
    pub fn with_operations(
        mut self,
        operations: Vec<(&str, Operation)>,
    ) -> Self {
        for (method, operation) in operations {
            match method.to_lowercase().as_str() {
                "get" => self.get = Some(operation),
                "post" => self.post = Some(operation),
                "put" => self.put = Some(operation),
                "delete" => self.delete = Some(operation),
                "patch" => self.patch = Some(operation),
                "options" => self.options = Some(operation),
                "head" => self.head = Some(operation),
                "trace" => self.trace = Some(operation),
                _ => {} // Ignore unknown methods
            }
        }
        self
    }
}

// Additional convenience methods for OpenAPIV3
impl OpenAPIV3 {
    /// Add a single path
    pub fn add_path(mut self, path: impl Into<String>, path_item: PathItem) -> Self {
        self.paths.insert(path.into(), path_item);
        self
    }

    /// Add multiple paths at once
    pub fn add_paths(mut self, paths: Vec<(impl Into<String>, PathItem)>) -> Self {
        for (path, path_item) in paths {
            self.paths.insert(path.into(), path_item);
        }
        self
    }

    /// Add a single server
    pub fn add_server(mut self, server: Server) -> Self {
        self.servers.get_or_insert_with(Vec::new).push(server);
        self
    }

    /// Add a tag
    pub fn add_tag(mut self, tag: Tag) -> Self {
        self.tags.get_or_insert_with(Vec::new).push(tag);
        self
    }
}

// Implement convenient JSON serialization methods
macro_rules! impl_serde_json {
    ($($st:ty,)+) => {
        $(
        impl $st {
            pub fn to_string(&self) -> String {
                serde_json::to_string(&self).unwrap()
            }
            pub fn to_value(&self) -> serde_json::Value {
                serde_json::to_value(&self).unwrap()
            }
        }
        )+
    };
}

impl_serde_json! {
    OpenAPIV3, Info, Contact, License, Server, ServerVariable, Components, PathItem,
    Operation, ExternalDocumentation, ParameterIn, Parameter, RequestBody, MediaType,
    Encoding, Responses, Response, Callback, Example, Link, Header, Tag, Reference,
    Schema, Discriminator,
}