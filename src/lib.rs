use std::collections::BTreeMap;

pub enum Referentable<T> {
    Data(T),
    Reference,
}

/// the root document object of openAPI v3.0
pub struct OpenAPIV3 {
    openapi: String,
    info: Info,
    servers: Option<Vec<Server>>,
    paths: BTreeMap<String, PathItem>,
    components: Option<Components>,
    security: Option<Vec<SecurityRequirement>>,
    tags: Option<Vec<Tag>>,
    external_docs: Option<ExternalDocumentation>,
}

pub struct Info {
    title: String,
    description: Option<String>,
    terms_Of_Service: Option<String>,
    contact: Option<Contact>,
    license: Option<License>,
    version: String,
}

pub struct Contact {
    name: Option<String>,
    url: Option<String>,
    email: Option<String>,
}

pub struct License {
    name: String,
    url: Option<String>,
}

pub struct Server {
    url: String,
    description: Option<String>,
    variables: BTreeMap<String, ServerVariable>,
}

pub struct ServerVariable {
    _enum: Option<Vec<String>>,
    default: String,
    description: Option<String>,
}

pub struct Components {
    schemas: BTreeMap<String, Referentable<Schema>>,
    responses: BTreeMap<String, Referentable<Response>>,
    parammeters: BTreeMap<String, Referentable<Parameter>>,
    examples: BTreeMap<String, Referentable<Example>>,
    request_bodies: BTreeMap<String, Referentable<RequestBody>>,
    headers: BTreeMap<String, Referentable<Header>>,
    security_schemes: BTreeMap<String, Referentable<SecurityScheme>>,
    links: BTreeMap<String, Referentable<Link>>,
    callbacks: BTreeMap<String, Referentable<Callback>>,
}

pub struct PathItem {
    _ref: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    get: Option<Operation>,
    put: Option<Operation>,
    post: Option<Operation>,
    delete: Option<Operation>,
    options: Option<Operation>,
    head: Option<Operation>,
    patch: Option<Operation>,
    trace: Option<Operation>,
    servers: Option<Vec<Server>>,
    parameters: Option<Vec<Referentable<Parameter>>>,
}

pub struct Operation {
    tag: Option<Vec<String>>,
    summary: Option<String>,
    description: Option<String>,
    external_docs: Option<ExternalDocumentation>,
    operationId: Option<String>,
    parameters: Option<Vec<Referentable<Parameter>>>,
    requestBody: Option<Referentable<RequestBody>>,
    reponse: Responses,
    callbacks: BTreeMap<String, Referentable<Callback>>,
    deprecated: Option<bool>,
    security: Option<Vec<SecurityRequirement>>,
    servers: Option<Vec<Server>>,
}

pub struct ExternalDocumentation {
    description: Option<String>,
    url: String,
}

pub enum ParameterIn {
    Query,
    Header,
    Path,
    Cookie,
}
pub struct Parameter {
    name: String,
    _in: ParameterIn,
    description: Option<String>,
    required: Option<bool>,
    deprecated: Option<bool>,
    allow_empty_value: Option<bool>,
    style: Option<String>,
    explode: Option<bool>,
    allow_reserved: Option<bool>,
    schema: Option<Referentable<Schema>>,
    // todo example
    examples: Option<BTreeMap<String, Referentable<Example>>>,
    content: Option<BTreeMap<String, MediaType>>,
}

pub struct RequestBody {
    description: Option<String>,
    content: BTreeMap<String, MediaType>,
    required: Option<bool>,
}

pub struct MediaType {
    schema: Referentable<Schema>,
    // todo example
    examples: Option<BTreeMap<String, Referentable<Example>>>,
    encoding: Option<BTreeMap<String, Encoding>>,
}
pub struct Encoding {
    content_type: Option<String>,
    headers: Option<BTreeMap<String, Referentable<Header>>>,
    style: Option<String>,
    explode: Option<bool>,
    allowReserved: Option<bool>,
}

pub struct Responses {
    default: Option<Referentable<Response>>,
    data: BTreeMap<String, Referentable<Response>>,
}

pub struct Response {
    description: String,
    headers: Option<BTreeMap<String, Referentable<Header>>>,
    content: Option<BTreeMap<String, MediaType>>,
    links: Option<BTreeMap<String, Referentable<Link>>>,
}

pub struct Callback {
    data: BTreeMap<String, PathItem>,
}

pub struct Example {
    summary: Option<String>,
    description: Option<String>,
    // todo value,
    external_value: Option<String>,
}
pub struct Any {
    // todo
}
pub struct Link {
    operation_ref: Option<String>,
    operation_id: Option<String>,
    parameters: Option<BTreeMap<String, Any>>,
    request_body: Option<Any>,
    description:Option<String>,
    server: Option<Server>
}
pub struct Header {

    _in: ParameterIn, // todo must be header
    description: Option<String>,
    required: Option<bool>,
    deprecated: Option<bool>,
    allow_empty_value: Option<bool>,
    style: Option<String>,
    explode: Option<bool>,
    allow_reserved: Option<bool>,
    schema: Option<Referentable<Schema>>,
    // todo example
    examples: Option<BTreeMap<String, Referentable<Example>>>,
    content: Option<BTreeMap<String, MediaType>>,
}

pub struct Tag {
    name: String,
    description: Option<String>,
    external_docs: Option<ExternalDocumentation>,
}

pub struct Reference {
    _ref: String
}
pub struct Schema {
    // todo
}

pub struct Discriminator {
    property_name: String,
    maapping: Option<BTreeMap<String,String>>
}

pub enum SecurityType{
    ApiKey,
    Http,
    Oauth2,
    OpenIdConnect
}

pub struct SecurityScheme {
    _type: SecurityType,
    description: Option<String>,
    name:String,
    _in: ParameterIn,
    scheme: String,
    bearer_format: Option<String>,
    flows: OauthFlows,
    open_id_connect_url:String
}

// todo should be enum
pub struct OauthFlows {
    implicit: Option<OauthFlow>,
    password: Option<OauthFlow>,
    client_credentials: Option<OauthFlow>,
    authorization_code: Option<OauthFlow>
}

pub struct OauthFlow {
    authorzation_url: String,
    token_url: String,
    refresh_url: Option<String>,
    scopes: BTreeMap<String,String>
}
pub struct SecurityRequirement {
    data: BTreeMap<String,Vec<String>>
}

