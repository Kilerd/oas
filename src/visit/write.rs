//! Mutable traversal of the strict model.

use std::collections::BTreeMap;
use std::convert::Infallible;

use super::{ComponentKind, ComponentRef, RefSiteMut, VisitMut};
use crate::security::{OauthFlow, OauthFlows, SecurityRequirement, SecurityScheme, SecurityType};
use crate::{
    Callback, Components, Contact, Encoding, Example, ExternalDocumentation, Header, Info, License,
    Link, MediaType, OpenAPIV3, Operation, Parameter, PathItem, Referenceable, RequestBody,
    Response, Responses, Schema, SchemaValue, Server, ServerVariable, Tag,
};

pub(crate) fn document<V: VisitMut + ?Sized>(document: &mut OpenAPIV3, visit: &mut V) {
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
    for server in servers.iter_mut().flatten() {
        server_node(visit, server);
    }
    for item in paths.values_mut() {
        path_item(visit, item);
    }
    if let Some(components) = components {
        components_node(visit, components);
    }
    security_requirements(visit, security.as_deref_mut());
    for tag in tags.iter_mut().flatten() {
        tag_node(visit, tag);
    }
    if let Some(docs) = external_docs {
        external_docs_node(visit, docs);
    }
}

fn info_node<V: VisitMut + ?Sized>(visit: &mut V, info: &mut Info) {
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

fn contact_node<V: VisitMut + ?Sized>(visit: &mut V, contact: &mut Contact) {
    let Contact {
        name: _,
        url: _,
        email: _,
        extras,
    } = contact;
    visit.extensions(extras);
}

fn license_node<V: VisitMut + ?Sized>(visit: &mut V, license: &mut License) {
    let License {
        name: _,
        url: _,
        extras,
    } = license;
    visit.extensions(extras);
}

fn server_node<V: VisitMut + ?Sized>(visit: &mut V, server: &mut Server) {
    let Server {
        url: _,
        description: _,
        variables,
        extras,
    } = server;
    visit.extensions(extras);
    for variable in variables.iter_mut().flat_map(BTreeMap::values_mut) {
        server_variable_node(visit, variable);
    }
}

fn server_variable_node<V: VisitMut + ?Sized>(visit: &mut V, variable: &mut ServerVariable) {
    let ServerVariable {
        _enum: _,
        default: _,
        description: _,
        extras,
    } = variable;
    visit.extensions(extras);
}

pub(crate) fn components_node<V: VisitMut + ?Sized>(visit: &mut V, components: &mut Components) {
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
    for value in schemas.iter_mut().flat_map(BTreeMap::values_mut) {
        schema_value(visit, value);
    }
    for node in responses.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, node, ComponentKind::Responses, |data, visit| {
            response_node(visit, data)
        });
    }
    for node in parameters.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, node, ComponentKind::Parameters, |data, visit| {
            parameter_node(visit, data)
        });
    }
    for node in examples.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, node, ComponentKind::Examples, |data, visit| {
            example_node(visit, data)
        });
    }
    for node in request_bodies.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, node, ComponentKind::RequestBodies, |data, visit| {
            request_body_node(visit, data)
        });
    }
    for node in headers.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, node, ComponentKind::Headers, |data, visit| {
            header_node(visit, data)
        });
    }
    for node in security_schemes.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(
            visit,
            node,
            ComponentKind::SecuritySchemes,
            |data, visit| security_scheme_node(visit, data),
        );
    }
    for node in links.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, node, ComponentKind::Links, |data, visit| {
            link_node(visit, data)
        });
    }
    for node in callbacks.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, node, ComponentKind::Callbacks, |data, visit| {
            callback_node(visit, data)
        });
    }
    for item in path_items.iter_mut().flat_map(BTreeMap::values_mut) {
        path_item(visit, item);
    }
}

pub(crate) fn component<V: VisitMut + ?Sized>(
    components: &mut Components,
    component: &ComponentRef,
    visit: &mut V,
) {
    match component.kind {
        ComponentKind::Schemas => {
            if let Some(value) = components
                .schemas
                .as_mut()
                .and_then(|group| group.get_mut(&component.name))
            {
                schema_value(visit, value);
            }
        }
        ComponentKind::Responses => {
            if let Some(value) = components
                .responses
                .as_mut()
                .and_then(|group| group.get_mut(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    response_node(visit, data)
                });
            }
        }
        ComponentKind::Parameters => {
            if let Some(value) = components
                .parameters
                .as_mut()
                .and_then(|group| group.get_mut(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    parameter_node(visit, data)
                });
            }
        }
        ComponentKind::Examples => {
            if let Some(value) = components
                .examples
                .as_mut()
                .and_then(|group| group.get_mut(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    example_node(visit, data)
                });
            }
        }
        ComponentKind::RequestBodies => {
            if let Some(value) = components
                .request_bodies
                .as_mut()
                .and_then(|group| group.get_mut(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    request_body_node(visit, data)
                });
            }
        }
        ComponentKind::Headers => {
            if let Some(value) = components
                .headers
                .as_mut()
                .and_then(|group| group.get_mut(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    header_node(visit, data)
                });
            }
        }
        ComponentKind::SecuritySchemes => {
            if let Some(value) = components
                .security_schemes
                .as_mut()
                .and_then(|group| group.get_mut(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    security_scheme_node(visit, data)
                });
            }
        }
        ComponentKind::Links => {
            if let Some(value) = components
                .links
                .as_mut()
                .and_then(|group| group.get_mut(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    link_node(visit, data)
                });
            }
        }
        ComponentKind::Callbacks => {
            if let Some(value) = components
                .callbacks
                .as_mut()
                .and_then(|group| group.get_mut(&component.name))
            {
                referenceable(visit, value, component.kind, |data, visit| {
                    callback_node(visit, data)
                });
            }
        }
        ComponentKind::PathItems => {
            if let Some(value) = components
                .path_items
                .as_mut()
                .and_then(|group| group.get_mut(&component.name))
            {
                path_item(visit, value);
            }
        }
    }
}

pub(crate) fn path_item<V: VisitMut + ?Sized>(visit: &mut V, item: &mut PathItem) {
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
    if let Some(target) = _ref.as_mut() {
        visit.reference(RefSiteMut {
            kind: ComponentKind::PathItems,
            target,
            extras: Some(extras),
            dynamic: false,
        });
    }
    for op in [get, put, post, delete, options, head, patch, trace]
        .into_iter()
        .filter_map(|operation| operation.as_deref_mut())
    {
        operation(visit, op);
    }
    for server in servers.iter_mut().flatten() {
        server_node(visit, server);
    }
    for parameter in parameters.iter_mut().flatten() {
        referenceable(
            visit,
            parameter,
            ComponentKind::Parameters,
            |data, visit| parameter_node(visit, data),
        );
    }
}

pub(crate) fn operation<V: VisitMut + ?Sized>(visit: &mut V, operation: &mut Operation) {
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
    for parameter in parameters.iter_mut().flatten() {
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
    for callback in callbacks.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, callback, ComponentKind::Callbacks, |data, visit| {
            callback_node(visit, data)
        });
    }
    security_requirements(visit, security.as_deref_mut());
    for server in servers.iter_mut().flatten() {
        server_node(visit, server);
    }
}

fn external_docs_node<V: VisitMut + ?Sized>(visit: &mut V, docs: &mut ExternalDocumentation) {
    let ExternalDocumentation {
        description: _,
        url: _,
        extras,
    } = docs;
    visit.extensions(extras);
}

fn parameter_node<V: VisitMut + ?Sized>(visit: &mut V, parameter: &mut Parameter) {
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
    for example in examples.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, example, ComponentKind::Examples, |data, visit| {
            example_node(visit, data)
        });
    }
    for media in content.iter_mut().flat_map(BTreeMap::values_mut) {
        media_node(visit, media);
    }
}

fn request_body_node<V: VisitMut + ?Sized>(visit: &mut V, body: &mut RequestBody) {
    let RequestBody {
        description: _,
        required: _,
        content,
        extras,
    } = body;
    visit.extensions(extras);
    for media in content.values_mut() {
        media_node(visit, media);
    }
}

fn media_node<V: VisitMut + ?Sized>(visit: &mut V, media: &mut MediaType) {
    let MediaType {
        schema,
        item_schema,
        example: _,
        examples,
        encoding,
        extras,
    } = media;
    visit.extensions(extras);
    for schema in schema.iter_mut().chain(item_schema.iter_mut()) {
        schema_value(visit, schema);
    }
    for example in examples.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, example, ComponentKind::Examples, |data, visit| {
            example_node(visit, data)
        });
    }
    for encoding in encoding.iter_mut().flat_map(BTreeMap::values_mut) {
        encoding_node(visit, encoding);
    }
}

fn encoding_node<V: VisitMut + ?Sized>(visit: &mut V, encoding: &mut Encoding) {
    let Encoding {
        content_type: _,
        headers,
        style: _,
        explode: _,
        allow_reserved: _,
        extras,
    } = encoding;
    visit.extensions(extras);
    for header in headers.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, header, ComponentKind::Headers, |data, visit| {
            header_node(visit, data)
        });
    }
}

fn responses_node<V: VisitMut + ?Sized>(visit: &mut V, responses: &mut Responses) {
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
    for response in data.values_mut() {
        referenceable(visit, response, ComponentKind::Responses, |data, visit| {
            response_node(visit, data)
        });
    }
}

fn response_node<V: VisitMut + ?Sized>(visit: &mut V, response: &mut Response) {
    let Response {
        description: _,
        headers,
        content,
        links,
        extras,
    } = response;
    visit.extensions(extras);
    for header in headers.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, header, ComponentKind::Headers, |data, visit| {
            header_node(visit, data)
        });
    }
    for media in content.iter_mut().flat_map(BTreeMap::values_mut) {
        media_node(visit, media);
    }
    for link in links.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, link, ComponentKind::Links, |data, visit| {
            link_node(visit, data)
        });
    }
}

fn callback_node<V: VisitMut + ?Sized>(visit: &mut V, callback: &mut Callback) {
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
        visit.reference(RefSiteMut {
            kind: ComponentKind::Callbacks,
            target,
            extras: Some(extras),
            dynamic: false,
        });
    }
    for item in data.values_mut() {
        path_item(visit, item);
    }
}

fn example_node<V: VisitMut + ?Sized>(visit: &mut V, example: &mut Example) {
    let Example {
        summary: _,
        description: _,
        value: _,
        external_value: _,
        extras,
    } = example;
    visit.extensions(extras);
}

fn link_node<V: VisitMut + ?Sized>(visit: &mut V, link: &mut Link) {
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

fn header_node<V: VisitMut + ?Sized>(visit: &mut V, header: &mut Header) {
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
    for example in examples.iter_mut().flat_map(BTreeMap::values_mut) {
        referenceable(visit, example, ComponentKind::Examples, |data, visit| {
            example_node(visit, data)
        });
    }
    for media in content.iter_mut().flat_map(BTreeMap::values_mut) {
        media_node(visit, media);
    }
}

fn tag_node<V: VisitMut + ?Sized>(visit: &mut V, tag: &mut Tag) {
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

fn security_scheme_node<V: VisitMut + ?Sized>(visit: &mut V, scheme: &mut SecurityScheme) {
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

fn oauth_flows_node<V: VisitMut + ?Sized>(visit: &mut V, flows: &mut OauthFlows) {
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

fn oauth_flow_node<V: VisitMut + ?Sized>(visit: &mut V, flow: &mut OauthFlow) {
    let OauthFlow {
        authorization_url: _,
        token_url: _,
        refresh_url: _,
        scopes: _,
        extras,
    } = flow;
    visit.extensions(extras);
}

fn security_requirements<V: VisitMut + ?Sized>(
    visit: &mut V,
    mut requirements: Option<&mut [SecurityRequirement]>,
) {
    for requirement in requirements.iter_mut().flat_map(|list| list.iter_mut()) {
        let SecurityRequirement { data } = requirement;
        let mut renames = Vec::new();
        for name in data.keys() {
            if let Some(new_name) = visit.security_scheme(name) {
                if new_name != *name {
                    renames.push((name.clone(), new_name));
                }
            }
        }
        for (from, to) in renames {
            if let Some(scopes) = data.remove(&from) {
                data.insert(to, scopes);
            }
        }
    }
}

fn referenceable<T, V: VisitMut + ?Sized>(
    visit: &mut V,
    node: &mut Referenceable<T>,
    kind: ComponentKind,
    inline: impl FnOnce(&mut T, &mut V),
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
            visit.reference(RefSiteMut {
                kind,
                target: _ref,
                extras: Some(extras),
                dynamic: false,
            });
        }
        Referenceable::Data(data) => inline(data, visit),
    }
}

pub(crate) fn schema_value<V: VisitMut + ?Sized>(visit: &mut V, value: &mut SchemaValue) {
    match value {
        SchemaValue::Boolean(_) => {}
        SchemaValue::Object(schema) => schema_node(visit, schema),
        #[cfg(feature = "lossless-schema-keywords")]
        SchemaValue::Invalid(_) => {}
    }
}

fn schema_node<V: VisitMut + ?Sized>(visit: &mut V, schema: &mut Schema) {
    visit.schema(schema);
    visit.extensions(&mut schema.extras);
    #[cfg(feature = "lossless-schema-keywords")]
    if let Some(docs) = schema.external_docs.as_mut() {
        visit.extensions(&mut docs.extras);
    }
    if let Some(target) = schema._ref.as_mut() {
        visit.reference(RefSiteMut {
            kind: ComponentKind::Schemas,
            target,
            extras: Some(&mut schema.extras),
            dynamic: false,
        });
    }
    for key in ["$dynamicRef", "$recursiveRef"] {
        if let Some(crate::Any::String(target)) = schema.extras.get_mut(key) {
            visit.reference(RefSiteMut {
                kind: ComponentKind::Schemas,
                target,
                extras: None,
                dynamic: true,
            });
        }
    }
    schema_mapping(visit, schema);
    let _ = schema.try_for_each_child_mut_with_keyword(|_, child| {
        schema_value(visit, child);
        Ok::<(), Infallible>(())
    });
    super::complete_schema(schema);
}

fn schema_mapping<V: VisitMut + ?Sized>(visit: &mut V, schema: &mut Schema) {
    #[cfg(feature = "lossless-schema-keywords")]
    {
        if let Some(discriminator) = schema.discriminator.as_mut() {
            let crate::lossless_component::Discriminator {
                property_name: _,
                mapping,
                extras,
            } = discriminator;
            visit.extensions(extras);
            if let crate::lossless_component::Field::Value(mapping) = mapping {
                for field in mapping.values_mut() {
                    if let crate::lossless_component::Field::Value(target) = field {
                        visit.reference(RefSiteMut {
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
        .get_mut("discriminator")
        .and_then(crate::Any::as_object_mut)
        .and_then(|discriminator| discriminator.get_mut("mapping"))
        .and_then(crate::Any::as_object_mut)
    {
        for value in mapping.values_mut() {
            if let crate::Any::String(target) = value {
                visit.reference(RefSiteMut {
                    kind: ComponentKind::Schemas,
                    target,
                    extras: None,
                    dynamic: false,
                });
            }
        }
    }
}
