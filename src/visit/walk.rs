//! One definition of the read-only and mutable walkers.
//!
//! The macro parameter `[$($mutability)?]` is empty for the read-only walker
//! and `[mut]` for the mutable one. Every `& $($mutability)? T` below is then
//! `&T` or `&mut T`. The remaining parameters are the two choices a node walk
//! cannot spell with mutability alone: immutable or mutable accessor methods
//! (`iter`/`iter_mut`, `get`/`get_mut`, ...), and the helpers that call the
//! visitor method with a mode-specific signature.
//!
//! Both invocations expand to ordinary modules, each with one copy of every
//! node walk. A node is destructured in full exactly once, in the macro body.

use std::collections::BTreeMap;

use crate::Any;

fn read_dynamic<'a>(extras: &'a BTreeMap<String, Any>, key: &str) -> Option<&'a str> {
    extras.get(key).and_then(Any::as_str)
}

fn write_dynamic<'a>(extras: &'a mut BTreeMap<String, Any>, key: &str) -> Option<&'a mut String> {
    match extras.get_mut(key) {
        Some(Any::String(target)) => Some(target),
        _ => None,
    }
}

/// Report extension maps nested in unmodeled schema objects.
///
/// `xml` is never a typed field, and `discriminator` / `externalDocs` are
/// typed only with `lossless-schema-keywords`. Their value objects stay in the
/// schema's unmodeled map, so their x-* keys are reported as the extension map
/// of the object they belong to. The read-only form copies the entries; the
/// mutable form copies them back in their original order after the visitor has
/// had a chance to remove or edit entries.
fn read_nested_extensions<V: crate::visit::Visit + ?Sized>(visit: &mut V, schema: &crate::Schema) {
    for key in ["xml", "discriminator", "externalDocs"] {
        if let Some(Any::Object(map)) = schema.extras.get(key) {
            let entries: BTreeMap<String, Any> = map
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            visit.extensions(&entries);
        }
    }
}

fn write_nested_extensions<V: crate::visit::VisitMut + ?Sized>(
    visit: &mut V,
    schema: &mut crate::Schema,
) {
    for key in ["xml", "discriminator", "externalDocs"] {
        let Some(Any::Object(map)) = schema.extras.get_mut(key) else {
            continue;
        };
        let entries: Vec<(String, Any)> = std::mem::take(map).into_iter().collect();
        let mut fields: BTreeMap<String, Any> = entries
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        visit.extensions(&mut fields);
        for (key, _) in entries {
            if let Some(value) = fields.remove(&key) {
                map.insert(key, value);
            }
        }
        for (key, value) in fields {
            map.insert(key, value);
        }
    }
}

/// Generates one walker module. The two invocations below differ only in the
/// accessors and visitor traits they name.
///
/// Parameters: module, visitor trait, reference site type, mutability token,
/// iterator, map-values accessor, `Option::as_deref`, `Option::as_ref`,
/// `BTreeMap::get`, `Value::as_object`, `Field::as_value`, schema child
/// visitor, dynamic-reference helper, nested-extension helper.
macro_rules! walkers {
    (
        $module:ident,
        $visitor:ident,
        $site:ident,
        [$($mutability:tt)?],
        $get:ident,
        $as_object:ident,
        $as_ref:ident,
        $child:ident,
        $dynamic:ident,
        $nested:ident $(,)?
    ) => {
        pub(crate) mod $module {
            use std::convert::Infallible;

            use crate::security::{
                OauthFlow, OauthFlows, SecurityRequirement, SecurityScheme, SecurityType,
            };
            use crate::visit::ComponentKind;
            use crate::{
                Any, Callback, Components, Contact, Encoding, Example, ExternalDocumentation, Header,
                Info, License, Link, MediaType, OpenAPIV3, Operation, Parameter, PathItem,
                Referenceable, RequestBody, Response, Responses, Schema, SchemaValue, Server,
                ServerVariable, Tag,
            };

            pub(crate) fn document<V: crate::visit::$visitor + ?Sized>(
                document: & $($mutability)? OpenAPIV3,
                visit: &mut V,
            ) {
                let OpenAPIV3 {
                    openapi: _,
                    info,
                    servers,
                    paths,
                    webhooks,
                    components,
                    security,
                    tags,
                    external_docs,
                    extras,
                } = document;
                if let Some(extras) = extras {
                    visit.extensions(extras);
                }
                info_node(visit, info);
                if let Some(values) = servers {
                    for server in values {
                        server_node(visit, server);
                    }
                }
                visit.extensions(& $($mutability)? paths.extras);
                for (_, item) in paths {
                    path_item(visit, item);
                }
                if let Some(webhooks) = webhooks {
                    for (_, item) in webhooks {
                        path_item(visit, item);
                    }
                }
                if let Some(components) = components {
                    components_node(visit, components);
                }
                security_requirements(visit, security);
                if let Some(values) = tags {
                    for tag in values {
                        tag_node(visit, tag);
                    }
                }
                if let Some(docs) = external_docs {
                    external_docs_node(visit, docs);
                }
            }

            fn info_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                info: & $($mutability)? Info,
            ) {
                let Info {
                    title: _,
                    description: _,
                    terms_of_service: _,
                    contact,
                    license,
                    version: _,
                    extras,
                } = info;
                visit.extensions(extras);
                if let Some(contact) = contact {
                    contact_node(visit, contact);
                }
                if let Some(license) = license {
                    license_node(visit, license);
                }
            }

            fn contact_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                contact: & $($mutability)? Contact,
            ) {
                let Contact {
                    name: _,
                    url: _,
                    email: _,
                    extras,
                } = contact;
                visit.extensions(extras);
            }

            fn license_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                license: & $($mutability)? License,
            ) {
                let License {
                    name: _,
                    url: _,
                    extras,
                } = license;
                visit.extensions(extras);
            }

            fn server_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                server: & $($mutability)? Server,
            ) {
                let Server {
                    url: _,
                    description: _,
                    variables,
                    extras,
                } = server;
                visit.extensions(extras);
                if let Some(map) = variables {
                    for (_, variable) in map {
                        server_variable_node(visit, variable);
                    }
                }
            }

            fn server_variable_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                variable: & $($mutability)? ServerVariable,
            ) {
                let ServerVariable {
                    _enum: _,
                    default: _,
                    description: _,
                    extras,
                } = variable;
                visit.extensions(extras);
            }

            pub(crate) fn components_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                components: & $($mutability)? Components,
            ) {
                let Components {
                    schemas,
                    responses,
                    parameters,
                    examples,
                    request_bodies,
                    headers,
                    security_schemes,
                    links,
                    callbacks,
                    path_items,
                    extras,
                } = components;
                visit.extensions(extras);
                if let Some(map) = schemas {
                    for (_, value) in map {
                        schema_value(visit, value);
                    }
                }
                if let Some(map) = responses {
                    for (_, node) in map {
                        referenceable(visit, node, ComponentKind::Responses, |data, visit| {
                            response_node(visit, data)
                        });
                    }
                }
                if let Some(map) = parameters {
                    for (_, node) in map {
                        referenceable(visit, node, ComponentKind::Parameters, |data, visit| {
                            parameter_node(visit, data)
                        });
                    }
                }
                if let Some(map) = examples {
                    for (_, node) in map {
                        referenceable(visit, node, ComponentKind::Examples, |data, visit| {
                            example_node(visit, data)
                        });
                    }
                }
                if let Some(map) = request_bodies {
                    for (_, node) in map {
                        referenceable(visit, node, ComponentKind::RequestBodies, |data, visit| {
                            request_body_node(visit, data)
                        });
                    }
                }
                if let Some(map) = headers {
                    for (_, node) in map {
                        referenceable(visit, node, ComponentKind::Headers, |data, visit| {
                            header_node(visit, data)
                        });
                    }
                }
                if let Some(map) = security_schemes {
                    for (_, node) in map {
                        referenceable(
                            visit,
                            node,
                            ComponentKind::SecuritySchemes,
                            |data, visit| security_scheme_node(visit, data),
                        );
                    }
                }
                if let Some(map) = links {
                    for (_, node) in map {
                        referenceable(visit, node, ComponentKind::Links, |data, visit| {
                            link_node(visit, data)
                        });
                    }
                }
                if let Some(map) = callbacks {
                    for (_, node) in map {
                        referenceable(visit, node, ComponentKind::Callbacks, |data, visit| {
                            callback_node(visit, data)
                        });
                    }
                }
                if let Some(map) = path_items {
                    for (_, item) in map {
                        path_item(visit, item);
                    }
                }
            }

            pub(crate) fn path_item<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                item: & $($mutability)? PathItem,
            ) {
                let PathItem {
                    _ref,
                    summary: _,
                    description: _,
                    get,
                    put,
                    post,
                    delete,
                    options,
                    head,
                    patch,
                    trace,
                    servers,
                    parameters,
                    extras,
                } = item;
                visit.extensions(extras);
                if let Some(target) = _ref {
                    visit.reference(crate::visit::$site {
                        kind: ComponentKind::PathItems,
                        target,
                        dynamic: false,
                        mapping: false,
                        operation_ref: false,
                    });
                }
                for op in [get, put, post, delete, options, head, patch, trace] {
                    if let Some(op) = op {
                        operation(visit, op);
                    }
                }
                if let Some(values) = servers {
                    for server in values {
                        server_node(visit, server);
                    }
                }
                if let Some(values) = parameters {
                    for parameter in values {
                        referenceable(
                            visit,
                            parameter,
                            ComponentKind::Parameters,
                            |data, visit| parameter_node(visit, data),
                        );
                    }
                }
            }

            pub(crate) fn operation<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                operation: & $($mutability)? Operation,
            ) {
                let Operation {
                    tags: _,
                    summary: _,
                    description: _,
                    external_docs,
                    operation_id: _,
                    parameters,
                    request_body,
                    responses,
                    callbacks,
                    deprecated: _,
                    security,
                    servers,
                    extras,
                } = operation;
                visit.extensions(extras);
                if let Some(docs) = external_docs {
                    external_docs_node(visit, docs);
                }
                if let Some(values) = parameters {
                    for parameter in values {
                        referenceable(
                            visit,
                            parameter,
                            ComponentKind::Parameters,
                            |data, visit| parameter_node(visit, data),
                        );
                    }
                }
                if let Some(body) = request_body {
                    referenceable(visit, body, ComponentKind::RequestBodies, |data, visit| {
                        request_body_node(visit, data)
                    });
                }
                responses_node(visit, responses);
                if let Some(map) = callbacks {
                    for (_, callback) in map {
                        referenceable(visit, callback, ComponentKind::Callbacks, |data, visit| {
                            callback_node(visit, data)
                        });
                    }
                }
                security_requirements(visit, security);
                if let Some(values) = servers {
                    for server in values {
                        server_node(visit, server);
                    }
                }
            }

            fn external_docs_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                docs: & $($mutability)? ExternalDocumentation,
            ) {
                let ExternalDocumentation {
                    description: _,
                    url: _,
                    extras,
                } = docs;
                visit.extensions(extras);
            }

            pub(crate) fn parameter_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                parameter: & $($mutability)? Parameter,
            ) {
                let Parameter {
                    name: _,
                    _in: _,
                    description: _,
                    required: _,
                    deprecated: _,
                    allow_empty_value: _,
                    style: _,
                    explode: _,
                    allow_reserved: _,
                    schema,
                    example: _,
                    examples,
                    content,
                    extras,
                } = parameter;
                visit.extensions(extras);
                if let Some(schema) = schema {
                    schema_value(visit, schema);
                }
                if let Some(map) = examples {
                    for (_, example) in map {
                        referenceable(visit, example, ComponentKind::Examples, |data, visit| {
                            example_node(visit, data)
                        });
                    }
                }
                if let Some(map) = content {
                    for (_, media) in map {
                        media_node(visit, media);
                    }
                }
            }

            pub(crate) fn request_body_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                body: & $($mutability)? RequestBody,
            ) {
                let RequestBody {
                    description: _,
                    required: _,
                    content,
                    extras,
                } = body;
                visit.extensions(extras);
                for (_, media) in content {
                    media_node(visit, media);
                }
            }

            fn media_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                media: & $($mutability)? MediaType,
            ) {
                let MediaType {
                    schema,
                    item_schema,
                    example: _,
                    examples,
                    encoding,
                    extras,
                } = media;
                visit.extensions(extras);
                if let Some(schema) = schema {
                    schema_value(visit, schema);
                }
                if let Some(item_schema) = item_schema {
                    schema_value(visit, item_schema);
                }
                if let Some(map) = examples {
                    for (_, example) in map {
                        referenceable(visit, example, ComponentKind::Examples, |data, visit| {
                            example_node(visit, data)
                        });
                    }
                }
                if let Some(map) = encoding {
                    for (_, encoding) in map {
                        encoding_node(visit, encoding);
                    }
                }
            }

            fn encoding_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                encoding: & $($mutability)? Encoding,
            ) {
                let Encoding {
                    content_type: _,
                    headers,
                    style: _,
                    explode: _,
                    allow_reserved: _,
                    extras,
                } = encoding;
                visit.extensions(extras);
                if let Some(map) = headers {
                    for (_, header) in map {
                        referenceable(visit, header, ComponentKind::Headers, |data, visit| {
                            header_node(visit, data)
                        });
                    }
                }
            }

            fn responses_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                responses: & $($mutability)? Responses,
            ) {
                let Responses {
                    default,
                    data,
                    extras,
                } = responses;
                visit.extensions(extras);
                if let Some(response) = default {
                    referenceable(visit, response, ComponentKind::Responses, |data, visit| {
                        response_node(visit, data)
                    });
                }
                for (_, response) in data {
                    referenceable(visit, response, ComponentKind::Responses, |data, visit| {
                        response_node(visit, data)
                    });
                }
            }

            pub(crate) fn response_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                response: & $($mutability)? Response,
            ) {
                let Response {
                    description: _,
                    headers,
                    content,
                    links,
                    extras,
                } = response;
                visit.extensions(extras);
                if let Some(map) = headers {
                    for (_, header) in map {
                        referenceable(visit, header, ComponentKind::Headers, |data, visit| {
                            header_node(visit, data)
                        });
                    }
                }
                if let Some(map) = content {
                    for (_, media) in map {
                        media_node(visit, media);
                    }
                }
                if let Some(map) = links {
                    for (_, link) in map {
                        referenceable(visit, link, ComponentKind::Links, |data, visit| {
                            link_node(visit, data)
                        });
                    }
                }
            }

            pub(crate) fn callback_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                callback: & $($mutability)? Callback,
            ) {
                let Callback {
                    #[cfg(feature = "lossless-schema-keywords")]
                    reference,
                    data,
                    extras,
                } = callback;
                visit.extensions(extras);
                #[cfg(feature = "lossless-schema-keywords")]
                if let crate::lossless_component::Field::Value(target) = reference {
                    visit.reference(crate::visit::$site {
                        kind: ComponentKind::Callbacks,
                        target,
                        dynamic: false,
                        mapping: false,
                        operation_ref: false,
                    });
                }
                for (_, item) in data {
                    path_item(visit, item);
                }
            }

            pub(crate) fn example_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                example: & $($mutability)? Example,
            ) {
                let Example {
                    summary: _,
                    description: _,
                    value: _,
                    external_value: _,
                    extras,
                } = example;
                visit.extensions(extras);
            }

            pub(crate) fn link_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                link: & $($mutability)? Link,
            ) {
                let Link {
                    operation_ref,
                    operation_id: _,
                    parameters: _,
                    request_body: _,
                    description: _,
                    server,
                    extras,
                } = link;
                visit.extensions(extras);
                if let Some(target) = operation_ref {
                    if target.starts_with("#/components/pathItems/") {
                        visit.reference(crate::visit::$site {
                            kind: ComponentKind::PathItems,
                            target,
                            dynamic: false,
                            mapping: false,
                            operation_ref: true,
                        });
                    }
                }
                if let Some(server) = server {
                    server_node(visit, server);
                }
            }

            pub(crate) fn header_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                header: & $($mutability)? Header,
            ) {
                let Header {
                    description: _,
                    required: _,
                    deprecated: _,
                    allow_empty_value: _,
                    style: _,
                    explode: _,
                    allow_reserved: _,
                    schema,
                    example: _,
                    examples,
                    content,
                    extras,
                } = header;
                visit.extensions(extras);
                if let Some(schema) = schema {
                    schema_value(visit, schema);
                }
                if let Some(map) = examples {
                    for (_, example) in map {
                        referenceable(visit, example, ComponentKind::Examples, |data, visit| {
                            example_node(visit, data)
                        });
                    }
                }
                if let Some(map) = content {
                    for (_, media) in map {
                        media_node(visit, media);
                    }
                }
            }

            fn tag_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                tag: & $($mutability)? Tag,
            ) {
                let Tag {
                    name: _,
                    description: _,
                    external_docs,
                    extras,
                } = tag;
                visit.extensions(extras);
                if let Some(docs) = external_docs {
                    external_docs_node(visit, docs);
                }
            }

            pub(crate) fn security_scheme_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                scheme: & $($mutability)? SecurityScheme,
            ) {
                let SecurityScheme {
                    _type,
                    description: _,
                    extras,
                } = scheme;
                visit.extensions(extras);
                match _type {
                    SecurityType::ApiKey { name: _, _in: _ } => {}
                    SecurityType::Http {
                        scheme: _,
                        bearer_format: _,
                    } => {}
                    SecurityType::Oauth2 { flows } => oauth_flows_node(visit, flows),
                    SecurityType::OpenIdConnect {
                        open_id_connect_url: _,
                    } => {}
                }
            }

            fn oauth_flows_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                flows: & $($mutability)? OauthFlows,
            ) {
                let OauthFlows {
                    implicit,
                    password,
                    client_credentials,
                    authorization_code,
                    extras,
                } = flows;
                visit.extensions(extras);
                for flow in [
                    implicit,
                    password,
                    client_credentials,
                    authorization_code,
                ]
                .into_iter()
                .flatten()
                {
                    oauth_flow_node(visit, flow);
                }
            }

            fn oauth_flow_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                flow: & $($mutability)? OauthFlow,
            ) {
                let OauthFlow {
                    authorization_url: _,
                    token_url: _,
                    refresh_url: _,
                    scopes: _,
                    extras,
                } = flow;
                visit.extensions(extras);
            }

            fn security_requirements<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                requirements: & $($mutability)? Option<Vec<SecurityRequirement>>,
            ) {
                if let Some(requirements) = requirements {
                    for requirement in requirements {
                        visit.security_requirement(requirement);
                    }
                }
            }

            pub(crate) fn referenceable<T, V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                node: & $($mutability)? Referenceable<T>,
                kind: ComponentKind,
                inline: impl FnOnce(& $($mutability)? T, &mut V),
            ) {
                match node {
                    Referenceable::Reference(reference) => {
                        let crate::Reference {
                            _ref,
                            summary: _,
                            description: _,
                            extras,
                        } = reference;
                        visit.extensions(extras);
                        visit.reference(crate::visit::$site {
                            kind,
                            target: _ref,
                            dynamic: false,
                            mapping: false,
                            operation_ref: false,
                        });
                    }
                    Referenceable::Data(data) => inline(data, visit),
                }
            }

            pub(crate) fn schema_value<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                value: & $($mutability)? SchemaValue,
            ) {
                match value {
                    SchemaValue::Boolean(_) => {}
                    SchemaValue::Object(schema) => schema_node(visit, schema),
                    #[cfg(feature = "lossless-schema-keywords")]
                    SchemaValue::Invalid(_) => {}
                }
            }

            fn schema_node<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                schema: & $($mutability)? Schema,
            ) {
                visit.schema(schema);
                visit.extensions(& $($mutability)? schema.extras);
                super::$nested(visit, schema);
                #[cfg(feature = "lossless-schema-keywords")]
                if let Some(docs) = & $($mutability)? schema.external_docs {
                    visit.extensions(& $($mutability)? docs.extras);
                }
                if let Some(target) = & $($mutability)? schema._ref {
                    visit.reference(crate::visit::$site {
                        kind: ComponentKind::Schemas,
                        target,
                        dynamic: false,
                        mapping: false,
                        operation_ref: false,
                    });
                }
                for key in ["$dynamicRef", "$recursiveRef"] {
                    if let Some(target) = super::$dynamic(& $($mutability)? schema.extras, key) {
                        visit.reference(crate::visit::$site {
                            kind: ComponentKind::Schemas,
                            target,
                            dynamic: true,
                            mapping: false,
                            operation_ref: false,
                        });
                    }
                }
                schema_mapping(visit, schema);
                let _ = schema.$child(|_, child| {
                    schema_value(visit, child);
                    Ok::<(), Infallible>(())
                });
                super::super::complete_schema(schema);
            }

            fn schema_mapping<V: crate::visit::$visitor + ?Sized>(
                visit: &mut V,
                schema: & $($mutability)? Schema,
            ) {
                #[cfg(feature = "lossless-schema-keywords")]
                {
                    if let Some(discriminator) = & $($mutability)? schema.discriminator {
                        let crate::lossless_component::Discriminator {
                            property_name: _,
                            mapping,
                            extras,
                        } = discriminator;
                        visit.extensions(extras);
                        if let crate::lossless_component::Field::Value(mapping) = mapping {
                            for (_, field) in mapping {
                                if let crate::lossless_component::Field::Value(target) = field {
                                    visit.reference(crate::visit::$site {
                                        kind: ComponentKind::Schemas,
                                        target,
                                        dynamic: false,
                                        mapping: true,
                                        operation_ref: false,
                                    });
                                }
                            }
                        }
                        return;
                    }
                }
                if let Some(mapping) = schema
                    .extras
                    .$get("discriminator")
                    .and_then(Any::$as_object)
                    .and_then(|discriminator| discriminator.$get("mapping"))
                    .and_then(Any::$as_object)
                {
                    for (_, value) in mapping {
                        if let Any::String(target) = value {
                            visit.reference(crate::visit::$site {
                                kind: ComponentKind::Schemas,
                                target,
                                dynamic: false,
                                mapping: true,
                                operation_ref: false,
                            });
                        }
                    }
                }
            }
        }
    };
}

// Read-only walker: immutable JSON accessors, `Visit`, and the read helpers
// for dynamic references and nested extension maps.
walkers!(
    read,
    Visit,
    RefSite,
    [],
    get,
    as_object,
    as_ref,
    try_for_each_child_with_keyword,
    read_dynamic,
    read_nested_extensions
);

// Mutable walker: mutable JSON accessors, `VisitMut`, and the write helpers.
walkers!(
    write,
    VisitMut,
    RefSiteMut,
    [mut],
    get_mut,
    as_object_mut,
    as_mut,
    try_for_each_child_mut_with_keyword,
    write_dynamic,
    write_nested_extensions
);
