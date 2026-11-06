//! Read-only traversal of the strict model.

use std::collections::BTreeMap;
use std::convert::Infallible;

use super::{ComponentKind, ComponentRef, RefSite, Visit};
use crate::security::{OauthFlow, OauthFlows, SecurityRequirement, SecurityScheme, SecurityType};
use crate::{
    Callback, Components, Contact, Encoding, Example, ExternalDocumentation, Header, Info, License,
    Link, MediaType, OpenAPIV3, Operation, Parameter, PathItem, Referenceable, RequestBody,
    Response, Responses, Schema, SchemaValue, Server, ServerVariable, Tag,
};

pub(crate) fn document<V: Visit + ?Sized>(document: &OpenAPIV3, visit: &mut V) {
    let OpenAPIV3 {
        openapi: _,
        info,
        servers,
        paths,
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
    for server in servers.iter().flatten() {
        server_node(visit, server);
    }
    for item in paths.values() {
        path_item(visit, item);
    }
    if let Some(components) = components {
        components_node(visit, components);
    }
    security_requirements(visit, security.as_deref());
    for tag in tags.iter().flatten() {
        tag_node(visit, tag);
    }
    if let Some(docs) = external_docs {
        external_docs_node(visit, docs);
    }
}

fn info_node<V: Visit + ?Sized>(visit: &mut V, info: &Info) {
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

fn contact_node<V: Visit + ?Sized>(visit: &mut V, contact: &Contact) {
    let Contact {
        name: _,
        url: _,
        email: _,
        extras,
    } = contact;
    visit.extensions(extras);
}

fn license_node<V: Visit + ?Sized>(visit: &mut V, license: &License) {
    let License {
        name: _,
        url: _,
        extras,
    } = license;
    visit.extensions(extras);
}

fn server_node<V: Visit + ?Sized>(visit: &mut V, server: &Server) {
    let Server {
        url: _,
        description: _,
        variables,
        extras,
    } = server;
    visit.extensions(extras);
    for variable in variables.iter().flat_map(BTreeMap::values) {
        server_variable_node(visit, variable);
    }
}

fn server_variable_node<V: Visit + ?Sized>(visit: &mut V, variable: &ServerVariable) {
    let ServerVariable {
        _enum: _,
        default: _,
        description: _,
        extras,
    } = variable;
    visit.extensions(extras);
}

fn components_node<V: Visit + ?Sized>(visit: &mut V, components: &Components) {
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
    for value in schemas.iter().flat_map(BTreeMap::values) {
        schema_value(visit, value);
    }
    for node in responses.iter().flat_map(BTreeMap::values) {
        referenceable(visit, node, ComponentKind::Responses, |data, visit| {
            response_node(visit, data)
        });
    }
    for node in parameters.iter().flat_map(BTreeMap::values) {
        referenceable(visit, node, ComponentKind::Parameters, |data, visit| {
            parameter_node(visit, data)
        });
    }
    for node in examples.iter().flat_map(BTreeMap::values) {
        referenceable(visit, node, ComponentKind::Examples, |data, visit| {
            example_node(visit, data)
        });
    }
    for node in request_bodies.iter().flat_map(BTreeMap::values) {
        referenceable(visit, node, ComponentKind::RequestBodies, |data, visit| {
            request_body_node(visit, data)
        });
    }
    for node in headers.iter().flat_map(BTreeMap::values) {
        referenceable(visit, node, ComponentKind::Headers, |data, visit| {
            header_node(visit, data)
        });
    }
    for node in security_schemes.iter().flat_map(BTreeMap::values) {
        referenceable(
            visit,
            node,
            ComponentKind::SecuritySchemes,
            |data, visit| security_scheme_node(visit, data),
        );
    }
    for node in links.iter().flat_map(BTreeMap::values) {
        referenceable(visit, node, ComponentKind::Links, |data, visit| {
            link_node(visit, data)
        });
    }
    for node in callbacks.iter().flat_map(BTreeMap::values) {
        referenceable(visit, node, ComponentKind::Callbacks, |data, visit| {
            callback_node(visit, data)
        });
    }
    for item in path_items.iter().flat_map(BTreeMap::values) {
        path_item(visit, item);
    }
}

pub(crate) fn component<V: Visit + ?Sized>(
    components: &Components,
    component: &ComponentRef,
    visit: &mut V,
) {
    match component.kind {
        ComponentKind::Schemas => {
            if let Some(value) = components
                .schemas
                .as_ref()
                .and_then(|group| group.get(&component.name))
            {
                schema_value(visit, value);
            }
        }
        ComponentKind::Responses => {
            if let Some(value) = components
                .responses
                .as_ref()
                .and_then(|group| group.get(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    response_node(visit, data)
                });
            }
        }
        ComponentKind::Parameters => {
            if let Some(value) = components
                .parameters
                .as_ref()
                .and_then(|group| group.get(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    parameter_node(visit, data)
                });
            }
        }
        ComponentKind::Examples => {
            if let Some(value) = components
                .examples
                .as_ref()
                .and_then(|group| group.get(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    example_node(visit, data)
                });
            }
        }
        ComponentKind::RequestBodies => {
            if let Some(value) = components
                .request_bodies
                .as_ref()
                .and_then(|group| group.get(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    request_body_node(visit, data)
                });
            }
        }
        ComponentKind::Headers => {
            if let Some(value) = components
                .headers
                .as_ref()
                .and_then(|group| group.get(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    header_node(visit, data)
                });
            }
        }
        ComponentKind::SecuritySchemes => {
            if let Some(value) = components
                .security_schemes
                .as_ref()
                .and_then(|group| group.get(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    security_scheme_node(visit, data)
                });
            }
        }
        ComponentKind::Links => {
            if let Some(value) = components
                .links
                .as_ref()
                .and_then(|group| group.get(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    link_node(visit, data)
                });
            }
        }
        ComponentKind::Callbacks => {
            if let Some(value) = components
                .callbacks
                .as_ref()
                .and_then(|group| group.get(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    callback_node(visit, data)
                });
            }
        }
        ComponentKind::PathItems => {
            if let Some(value) = components
                .path_items
                .as_ref()
                .and_then(|group| group.get(&component.name))
            {
                path_item(visit, value);
            }
        }
    }
}

pub(crate) fn path_item<V: Visit + ?Sized>(visit: &mut V, item: &PathItem) {
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
        visit.reference(RefSite {
            kind: ComponentKind::PathItems,
            target,
            extras: Some(extras),
            dynamic: false,
        });
    }
    for op in [get, put, post, delete, options, head, patch, trace]
        .into_iter()
        .flatten()
    {
        operation(visit, op);
    }
    for server in servers.iter().flatten() {
        server_node(visit, server);
    }
    for parameter in parameters.iter().flatten() {
        referenceable(
            visit,
            parameter,
            ComponentKind::Parameters,
            |data, visit| parameter_node(visit, data),
        );
    }
}

pub(crate) fn operation<V: Visit + ?Sized>(visit: &mut V, operation: &Operation) {
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
    for parameter in parameters.iter().flatten() {
        referenceable(
            visit,
            parameter,
            ComponentKind::Parameters,
            |data, visit| parameter_node(visit, data),
        );
    }
    if let Some(body) = request_body {
        referenceable(visit, body, ComponentKind::RequestBodies, |data, visit| {
            request_body_node(visit, data)
        });
    }
    responses_node(visit, responses);
    for callback in callbacks.iter().flat_map(BTreeMap::values) {
        referenceable(visit, callback, ComponentKind::Callbacks, |data, visit| {
            callback_node(visit, data)
        });
    }
    security_requirements(visit, security.as_deref());
    for server in servers.iter().flatten() {
        server_node(visit, server);
    }
}

fn external_docs_node<V: Visit + ?Sized>(visit: &mut V, docs: &ExternalDocumentation) {
    let ExternalDocumentation {
        description: _,
        url: _,
        extras,
    } = docs;
    visit.extensions(extras);
}

fn parameter_node<V: Visit + ?Sized>(visit: &mut V, parameter: &Parameter) {
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
    for example in examples.iter().flat_map(BTreeMap::values) {
        referenceable(visit, example, ComponentKind::Examples, |data, visit| {
            example_node(visit, data)
        });
    }
    for media in content.iter().flat_map(BTreeMap::values) {
        media_node(visit, media);
    }
}

fn request_body_node<V: Visit + ?Sized>(visit: &mut V, body: &RequestBody) {
    let RequestBody {
        description: _,
        required: _,
        content,
        extras,
    } = body;
    visit.extensions(extras);
    for media in content.values() {
        media_node(visit, media);
    }
}

fn media_node<V: Visit + ?Sized>(visit: &mut V, media: &MediaType) {
    let MediaType {
        schema,
        item_schema,
        example: _,
        examples,
        encoding,
        extras,
    } = media;
    visit.extensions(extras);
    for schema in schema.iter().chain(item_schema.iter()) {
        schema_value(visit, schema);
    }
    for example in examples.iter().flat_map(BTreeMap::values) {
        referenceable(visit, example, ComponentKind::Examples, |data, visit| {
            example_node(visit, data)
        });
    }
    for encoding in encoding.iter().flat_map(BTreeMap::values) {
        encoding_node(visit, encoding);
    }
}

fn encoding_node<V: Visit + ?Sized>(visit: &mut V, encoding: &Encoding) {
    let Encoding {
        content_type: _,
        headers,
        style: _,
        explode: _,
        allow_reserved: _,
        extras,
    } = encoding;
    visit.extensions(extras);
    for header in headers.iter().flat_map(BTreeMap::values) {
        referenceable(visit, header, ComponentKind::Headers, |data, visit| {
            header_node(visit, data)
        });
    }
}

fn responses_node<V: Visit + ?Sized>(visit: &mut V, responses: &Responses) {
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
    for response in data.values() {
        referenceable(visit, response, ComponentKind::Responses, |data, visit| {
            response_node(visit, data)
        });
    }
}

fn response_node<V: Visit + ?Sized>(visit: &mut V, response: &Response) {
    let Response {
        description: _,
        headers,
        content,
        links,
        extras,
    } = response;
    visit.extensions(extras);
    for header in headers.iter().flat_map(BTreeMap::values) {
        referenceable(visit, header, ComponentKind::Headers, |data, visit| {
            header_node(visit, data)
        });
    }
    for media in content.iter().flat_map(BTreeMap::values) {
        media_node(visit, media);
    }
    for link in links.iter().flat_map(BTreeMap::values) {
        referenceable(visit, link, ComponentKind::Links, |data, visit| {
            link_node(visit, data)
        });
    }
}

fn callback_node<V: Visit + ?Sized>(visit: &mut V, callback: &Callback) {
    #[cfg(feature = "lossless-schema-keywords")]
    let (reference, data, extras) = {
        let Callback {
            reference,
            data,
            extras,
        } = callback;
        (reference, data, extras)
    };
    #[cfg(not(feature = "lossless-schema-keywords"))]
    let (data, extras) = {
        let Callback { data, extras } = callback;
        (data, extras)
    };
    visit.extensions(extras);
    #[cfg(feature = "lossless-schema-keywords")]
    if let crate::lossless_component::Field::Value(target) = reference {
        visit.reference(RefSite {
            kind: ComponentKind::Callbacks,
            target,
            extras: Some(extras),
            dynamic: false,
        });
    }
    for item in data.values() {
        path_item(visit, item);
    }
}

fn example_node<V: Visit + ?Sized>(visit: &mut V, example: &Example) {
    let Example {
        summary: _,
        description: _,
        value: _,
        external_value: _,
        extras,
    } = example;
    visit.extensions(extras);
}

fn link_node<V: Visit + ?Sized>(visit: &mut V, link: &Link) {
    let Link {
        operation_ref: _,
        operation_id: _,
        parameters: _,
        request_body: _,
        description: _,
        server,
        extras,
    } = link;
    visit.extensions(extras);
    if let Some(server) = server {
        server_node(visit, server);
    }
}

fn header_node<V: Visit + ?Sized>(visit: &mut V, header: &Header) {
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
    for example in examples.iter().flat_map(BTreeMap::values) {
        referenceable(visit, example, ComponentKind::Examples, |data, visit| {
            example_node(visit, data)
        });
    }
    for media in content.iter().flat_map(BTreeMap::values) {
        media_node(visit, media);
    }
}

fn tag_node<V: Visit + ?Sized>(visit: &mut V, tag: &Tag) {
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

fn security_scheme_node<V: Visit + ?Sized>(visit: &mut V, scheme: &SecurityScheme) {
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

fn oauth_flows_node<V: Visit + ?Sized>(visit: &mut V, flows: &OauthFlows) {
    let OauthFlows {
        implicit,
        password,
        client_credentials,
        authorization_code,
        extras,
    } = flows;
    visit.extensions(extras);
    for flow in [implicit, password, client_credentials, authorization_code]
        .into_iter()
        .flatten()
    {
        oauth_flow_node(visit, flow);
    }
}

fn oauth_flow_node<V: Visit + ?Sized>(visit: &mut V, flow: &OauthFlow) {
    let OauthFlow {
        authorization_url: _,
        token_url: _,
        refresh_url: _,
        scopes: _,
        extras,
    } = flow;
    visit.extensions(extras);
}

fn security_requirements<V: Visit + ?Sized>(
    visit: &mut V,
    requirements: Option<&[SecurityRequirement]>,
) {
    for requirement in requirements.iter().flat_map(|list| list.iter()) {
        let SecurityRequirement { data } = requirement;
        for name in data.keys() {
            visit.security_scheme(name);
        }
    }
}

fn referenceable<T, V: Visit + ?Sized>(
    visit: &mut V,
    node: &Referenceable<T>,
    kind: ComponentKind,
    inline: impl FnOnce(&T, &mut V),
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
            visit.reference(RefSite {
                kind,
                target: _ref,
                extras: Some(extras),
                dynamic: false,
            });
        }
        Referenceable::Data(data) => inline(data, visit),
    }
}

pub(crate) fn schema_value<V: Visit + ?Sized>(visit: &mut V, value: &SchemaValue) {
    match value {
        SchemaValue::Boolean(_) => {}
        SchemaValue::Object(schema) => schema_node(visit, schema),
        #[cfg(feature = "lossless-schema-keywords")]
        SchemaValue::Invalid(_) => {}
    }
}

fn schema_node<V: Visit + ?Sized>(visit: &mut V, schema: &Schema) {
    visit.schema(schema);
    visit.extensions(&schema.extras);
    #[cfg(feature = "lossless-schema-keywords")]
    if let Some(docs) = &schema.external_docs {
        visit.extensions(&docs.extras);
    }
    if let Some(target) = &schema._ref {
        visit.reference(RefSite {
            kind: ComponentKind::Schemas,
            target,
            extras: Some(&schema.extras),
            dynamic: false,
        });
    }
    for key in ["$dynamicRef", "$recursiveRef"] {
        if let Some(target) = schema.extras.get(key).and_then(crate::Any::as_str) {
            visit.reference(RefSite {
                kind: ComponentKind::Schemas,
                target,
                extras: None,
                dynamic: true,
            });
        }
    }
    schema_mapping(visit, schema);
    let _ = schema.try_for_each_child_with_keyword(|_, child| {
        schema_value(visit, child);
        Ok::<(), Infallible>(())
    });
    super::complete_schema(schema);
}

fn schema_mapping<V: Visit + ?Sized>(visit: &mut V, schema: &Schema) {
    #[cfg(feature = "lossless-schema-keywords")]
    {
        if let Some(discriminator) = &schema.discriminator {
            let crate::lossless_component::Discriminator {
                property_name: _,
                mapping,
                extras,
            } = discriminator;
            visit.extensions(extras);
            if let crate::lossless_component::Field::Value(mapping) = mapping {
                for field in mapping.values() {
                    if let crate::lossless_component::Field::Value(target) = field {
                        visit.reference(RefSite {
                            kind: ComponentKind::Schemas,
                            target,
                            extras: None,
                            dynamic: false,
                        });
                    }
                }
            }
            return;
        }
    }
    if let Some(mapping) = schema
        .extras
        .get("discriminator")
        .and_then(crate::Any::as_object)
        .and_then(|discriminator| discriminator.get("mapping"))
        .and_then(crate::Any::as_object)
    {
        for target in mapping.values().filter_map(crate::Any::as_str) {
            visit.reference(RefSite {
                kind: ComponentKind::Schemas,
                target,
                extras: None,
                dynamic: false,
            });
        }
    }
}
