//! Traversal, reference graphs and reference rewriting for the strict model.
//!
//! [`Visit`] and [`VisitMut`] walk an [`OpenAPIV3`] document, an [`Operation`],
//! a [`PathItem`], or one component through [`Components::walk_component`].
//! They report the positions that hold references, every schema object, every
//! unmodeled/extension map, and every security requirement key. Literal data
//! (`example`, `examples` values, `default`, `enum`, `const`, extension values)
//! is never visited.
//!
//! References are reported with the [`ComponentKind`] of their *position*, so a
//! `$ref` in a response position is a response reference even when it points at
//! another component kind. A `$dynamicRef` or `$recursiveRef` whose value is a
//! string is reported with [`RefSite::dynamic`] set; [`RefGraph`] resolves any
//! such site to [`RefError::Dynamic`] instead of following it.
//!
//! Every node is destructured completely in the walkers. Adding a field to a
//! strict model type fails to compile in this module until the walk decides
//! whether the field holds references or schemas.
//!
//! ```
//! use oas::visit::{ComponentKind, RefSite, Visit};
//! use oas::{OpenAPIV3, Schema};
//!
//! #[derive(Default)]
//! struct Counts {
//!     schemas: usize,
//!     references: usize,
//! }
//!
//! impl Visit for Counts {
//!     fn schema(&mut self, _schema: &Schema) {
//!         self.schemas += 1;
//!     }
//!     fn reference(&mut self, site: RefSite<'_>) {
//!         if site.kind == ComponentKind::Schemas {
//!             self.references += 1;
//!         }
//!     }
//! }
//!
//! let mut doc: OpenAPIV3 = serde_json::from_value(serde_json::json!({
//!     "openapi": "3.1.0",
//!     "info": {"title": "t", "version": "1"},
//!     "paths": {},
//!     "components": {"schemas": {
//!         "A": {"$ref": "#/components/schemas/B"},
//!         "B": {"type": "string"}
//!     }}
//! }))
//! .unwrap();
//! let mut counts = Counts::default();
//! doc.walk(&mut counts);
//! assert_eq!((counts.schemas, counts.references), (2, 1));
//! ```
//!
//! The graph below is built once; closure and dependent questions are then
//! linear in the number of edges. Renaming only rewrites pointers that resolve
//! to a component in the map, and security requirement keys when the
//! `securitySchemes` component they name is renamed.
//!
//! ```
//! use std::collections::BTreeMap;
//! use oas::visit::{ComponentKind, ComponentRef};
//! use oas::OpenAPIV3;
//!
//! let mut doc: OpenAPIV3 = serde_json::from_value(serde_json::json!({
//!     "openapi": "3.1.0",
//!     "info": {"title": "t", "version": "1"},
//!     "paths": {"/x": {"get": {
//!         "security": [{"Bearer": []}],
//!         "responses": {"200": {"$ref": "#/components/responses/Ok"}}
//!     }}},
//!     "components": {
//!         "responses": {"Ok": {"description": "ok"}},
//!         "securitySchemes": {"Bearer": {"type": "http", "scheme": "bearer"}}
//!     }
//! }))
//! .unwrap();
//! let graph = doc.ref_graph();
//! let roots = doc.paths["/x"].get.as_ref().unwrap().refs();
//! let roots = graph.roots(roots).unwrap();
//! assert_eq!(roots.len(), 1);
//!
//! let auth = ComponentRef::new(ComponentKind::SecuritySchemes, "Bearer");
//! let ok = ComponentRef::new(ComponentKind::Responses, "Ok");
//! doc.rename_refs(&BTreeMap::from([
//!     (auth, "Auth".to_string()),
//!     (ok, "Success".to_string()),
//! ]));
//! let text = serde_json::to_string(&doc).unwrap();
//! assert!(text.contains("#/components/responses/Success"));
//! assert!(text.contains(r#""security":[{"Auth":[]}]"#));
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::json_pointer;
use crate::{Any, Components, OpenAPIV3, Operation, PathItem, Schema};

mod graph;
mod walk;

use walk::{read, write};

pub use graph::RefGraph;

/// One of the ten component groups under `components`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ComponentKind {
    Schemas,
    Responses,
    Parameters,
    Examples,
    RequestBodies,
    Headers,
    SecuritySchemes,
    Links,
    Callbacks,
    PathItems,
}

impl ComponentKind {
    /// Every component kind, in the order the specification lists them.
    pub const ALL: [Self; 10] = [
        Self::Schemas,
        Self::Responses,
        Self::Parameters,
        Self::Examples,
        Self::RequestBodies,
        Self::Headers,
        Self::SecuritySchemes,
        Self::Links,
        Self::Callbacks,
        Self::PathItems,
    ];

    /// The key of this group under `components`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Schemas => "schemas",
            Self::Responses => "responses",
            Self::Parameters => "parameters",
            Self::Examples => "examples",
            Self::RequestBodies => "requestBodies",
            Self::Headers => "headers",
            Self::SecuritySchemes => "securitySchemes",
            Self::Links => "links",
            Self::Callbacks => "callbacks",
            Self::PathItems => "pathItems",
        }
    }

    /// The kind whose group key is `key`.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == key)
    }
}

/// A component named by its kind and its key under that group.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentRef {
    pub kind: ComponentKind,
    pub name: String,
}

impl ComponentRef {
    /// A reference to `name` in `kind`.
    pub fn new(kind: ComponentKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
        }
    }

    /// Parse exactly `#/components/{kind}/{name}`, decoding RFC 6901 escapes.
    ///
    /// A URL or relative file reference is [`RefError::External`]; another
    /// local pointer, a deeper pointer, or an unknown component group is
    /// [`RefError::OutsideComponents`]. Resolution against a document (whether
    /// the component exists and has the expected kind) is done by [`RefGraph`]
    /// and [`Components::get`].
    pub fn parse(target: &str) -> Result<Self, RefError> {
        if target.starts_with('$') {
            return Err(RefError::Dynamic);
        }
        let Some(pointer) = target.strip_prefix('#') else {
            return Err(RefError::External);
        };
        let tokens = json_pointer::tokens(pointer).map_err(|_| RefError::OutsideComponents)?;
        let [root, kind, name] =
            <[String; 3]>::try_from(tokens).map_err(|_| RefError::OutsideComponents)?;
        if root != "components" {
            return Err(RefError::OutsideComponents);
        }
        let kind = ComponentKind::from_key(&kind).ok_or(RefError::OutsideComponents)?;
        Ok(Self { kind, name })
    }

    /// The `#/components/{kind}/{name}` pointer for this component, with RFC
    /// 6901 escapes applied to `name`.
    pub fn to_pointer(&self) -> String {
        format!(
            "#/components/{}/{}",
            self.kind.key(),
            json_pointer::escape(&self.name)
        )
    }
}

impl fmt::Display for ComponentRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_pointer())
    }
}

/// Why a reference does not name a component of the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefError {
    /// A URL or a relative file reference, not a local pointer.
    External,
    /// A local pointer that is not exactly under `#/components/{kind}/{name}`.
    OutsideComponents,
    /// The component the pointer names does not exist.
    Missing(ComponentRef),
    /// The reference position expects another component kind.
    WrongKind {
        expected: ComponentKind,
        found: ComponentKind,
    },
    /// A `$dynamicRef` or `$recursiveRef`; these are not followed statically.
    Dynamic,
}

impl fmt::Display for RefError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::External => formatter.write_str("a reference outside the document"),
            Self::OutsideComponents => {
                formatter.write_str("a local pointer outside #/components/{kind}/{name}")
            }
            Self::Missing(component) => write!(formatter, "missing component {component}"),
            Self::WrongKind { expected, found } => write!(
                formatter,
                "expected a {} reference, found a {} reference",
                expected.key(),
                found.key()
            ),
            Self::Dynamic => formatter.write_str("a dynamic reference"),
        }
    }
}

impl std::error::Error for RefError {}

/// A reference a node writes directly: the kind of its position, the raw
/// target, and whether it is a dynamic reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectRef {
    pub kind: ComponentKind,
    pub target: String,
    pub dynamic: bool,
    /// True for a `discriminator.mapping` value.
    pub mapping: bool,
}

/// A read-only reference site.
#[derive(Debug, Clone, Copy)]
pub struct RefSite<'a> {
    /// The component kind of the position holding the reference.
    pub kind: ComponentKind,
    /// The raw target as written in the document.
    pub target: &'a str,
    /// The Reference Object's unmodeled fields, when the site is a Reference
    /// Object. A schema `$ref` reports the schema's unmodeled fields; a
    /// `discriminator.mapping` value and a dynamic reference report `None`.
    pub extras: Option<&'a BTreeMap<String, Any>>,
    /// True for a `$dynamicRef` or `$recursiveRef` with a string value.
    pub dynamic: bool,
    /// True for a `discriminator.mapping` value. A bare value (no `#` and no
    /// `/`) names `components.schemas`; the graph resolves it as such and a
    /// rename keeps it bare.
    pub mapping: bool,
}

/// A mutable reference site.
#[derive(Debug)]
pub struct RefSiteMut<'a> {
    /// The component kind of the position holding the reference.
    pub kind: ComponentKind,
    /// The target to rewrite.
    pub target: &'a mut String,
    /// The Reference Object's unmodeled fields, when the site is a Reference
    /// Object. A schema `$ref` reports the schema's unmodeled fields; a
    /// `discriminator.mapping` value and a dynamic reference report `None`.
    pub extras: Option<&'a mut BTreeMap<String, Any>>,
    /// True for a `$dynamicRef` or `$recursiveRef` with a string value.
    pub dynamic: bool,
    /// True for a `discriminator.mapping` value. A bare value (no `#` and no
    /// `/`) names `components.schemas`; the graph resolves it as such and a
    /// rename keeps it bare.
    pub mapping: bool,
}

/// Read-only visitor over the strict model. Every method does nothing by
/// default.
pub trait Visit {
    /// A reference in a position of `site.kind`.
    fn reference(&mut self, _site: RefSite<'_>) {}
    /// A Schema Object, parents before children.
    fn schema(&mut self, _schema: &Schema) {}
    /// The unmodeled and `x-*` keys of one object.
    fn extensions(&mut self, _extensions: &BTreeMap<String, Any>) {}
    /// One security requirement, whose keys name `securitySchemes`
    /// components.
    fn security_requirement(&mut self, _requirement: &crate::SecurityRequirement) {}
}

/// Mutable visitor over the strict model. Every method does nothing by default.
pub trait VisitMut {
    /// A reference in a position of `site.kind`.
    fn reference(&mut self, _site: RefSiteMut<'_>) {}
    /// A Schema Object, parents before children.
    fn schema(&mut self, _schema: &mut Schema) {}
    /// The unmodeled and `x-*` keys of one object.
    fn extensions(&mut self, _extensions: &mut BTreeMap<String, Any>) {}
    /// One security requirement, whose keys name `securitySchemes`
    /// components. Rebuilding `requirement.data` makes a rename collision
    /// explicit; the default leaves it unchanged.
    fn security_requirement(&mut self, _requirement: &mut crate::SecurityRequirement) {}
}

/// One component value, borrowed from its group.
#[derive(Debug, Clone, Copy)]
pub enum ComponentValue<'a> {
    Schema(&'a crate::SchemaValue),
    Response(&'a crate::Referenceable<crate::Response>),
    Parameter(&'a crate::Referenceable<crate::Parameter>),
    Example(&'a crate::Referenceable<crate::Example>),
    RequestBody(&'a crate::Referenceable<crate::RequestBody>),
    Header(&'a crate::Referenceable<crate::Header>),
    SecurityScheme(&'a crate::Referenceable<crate::SecurityScheme>),
    Link(&'a crate::Referenceable<crate::Link>),
    Callback(&'a crate::Referenceable<crate::Callback>),
    PathItem(&'a crate::PathItem),
}

impl ComponentValue<'_> {
    /// The kind of this component.
    pub fn kind(&self) -> ComponentKind {
        match self {
            Self::Schema(_) => ComponentKind::Schemas,
            Self::Response(_) => ComponentKind::Responses,
            Self::Parameter(_) => ComponentKind::Parameters,
            Self::Example(_) => ComponentKind::Examples,
            Self::RequestBody(_) => ComponentKind::RequestBodies,
            Self::Header(_) => ComponentKind::Headers,
            Self::SecurityScheme(_) => ComponentKind::SecuritySchemes,
            Self::Link(_) => ComponentKind::Links,
            Self::Callback(_) => ComponentKind::Callbacks,
            Self::PathItem(_) => ComponentKind::PathItems,
        }
    }

    /// The component as a JSON value.
    pub fn to_value(&self) -> Any {
        match self {
            Self::Schema(value) => serde_json::to_value(value),
            Self::Response(value) => serde_json::to_value(value),
            Self::Parameter(value) => serde_json::to_value(value),
            Self::Example(value) => serde_json::to_value(value),
            Self::RequestBody(value) => serde_json::to_value(value),
            Self::Header(value) => serde_json::to_value(value),
            Self::SecurityScheme(value) => serde_json::to_value(value),
            Self::Link(value) => serde_json::to_value(value),
            Self::Callback(value) => serde_json::to_value(value),
            Self::PathItem(value) => serde_json::to_value(value),
        }
        .expect("a component serializes")
    }
}

/// Components are equal when they have the same kind and the same JSON value.
impl PartialEq for ComponentValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.kind() == other.kind() && self.to_value() == other.to_value()
    }
}

impl Eq for ComponentValue<'_> {}

impl OpenAPIV3 {
    /// Visit every reference, schema, extension map and security requirement
    /// key of this document, including its operations and components.
    pub fn walk(&self, visitor: &mut impl Visit) {
        read::document(self, visitor);
    }

    /// The mutable form of [`OpenAPIV3::walk`].
    pub fn walk_mut(&mut self, visitor: &mut impl VisitMut) {
        write::document(self, visitor);
    }

    /// Build the component reference graph of this document. A document
    /// without components has an empty graph.
    pub fn ref_graph(&self) -> RefGraph {
        self.components
            .as_ref()
            .map(Components::ref_graph)
            .unwrap_or_default()
    }

    /// Rewrite every reference target through `renames`, and every security
    /// requirement key whose `securitySchemes` component is renamed.
    ///
    /// A pointer deeper than `#/components/{kind}/{name}` keeps its tail, so
    /// `.../Foo/properties/bar` follows a rename of `Foo`.
    pub fn rename_refs(&mut self, renames: &BTreeMap<ComponentRef, String>) {
        self.walk_mut(&mut Rename { renames });
    }
}

impl Operation {
    /// Visit every reference, schema, extension map and security requirement
    /// key of this operation, including callbacks and their path items.
    pub fn walk(&self, visitor: &mut impl Visit) {
        read::operation(visitor, self);
    }

    /// The mutable form of [`Operation::walk`].
    pub fn walk_mut(&mut self, visitor: &mut impl VisitMut) {
        write::operation(visitor, self);
    }

    /// The references this operation writes itself, in walk order.
    ///
    /// Security requirement keys are reported as `securitySchemes`
    /// references. Path-level parameters and the document's root `security`
    /// are not part of an operation's own roots.
    pub fn refs(&self) -> Vec<DirectRef> {
        let mut collect = Collect::default();
        read::operation(&mut collect, self);
        collect.0
    }

    /// Rewrite every reference target of this operation through `renames`,
    /// and every security requirement key whose scheme is renamed.
    pub fn rename_refs(&mut self, renames: &BTreeMap<ComponentRef, String>) {
        self.walk_mut(&mut Rename { renames });
    }
}

impl PathItem {
    /// Visit every reference, schema, extension map and security requirement
    /// key of this path item, including its operations.
    pub fn walk(&self, visitor: &mut impl Visit) {
        read::path_item(visitor, self);
    }

    /// The mutable form of [`PathItem::walk`].
    pub fn walk_mut(&mut self, visitor: &mut impl VisitMut) {
        write::path_item(visitor, self);
    }

    /// Rewrite every reference target of this path item through `renames`.
    pub fn rename_refs(&mut self, renames: &BTreeMap<ComponentRef, String>) {
        self.walk_mut(&mut Rename { renames });
    }
}

impl Components {
    /// Visit one component of this group by name, if it exists.
    pub fn walk_component(&self, component: &ComponentRef, visitor: &mut impl Visit) {
        read::component(self, component, visitor);
    }

    /// The mutable form of [`Components::walk_component`].
    pub fn walk_component_mut(&mut self, component: &ComponentRef, visitor: &mut impl VisitMut) {
        write::component(self, component, visitor);
    }

    /// Borrow one component by kind and name.
    pub fn get(&self, component: &ComponentRef) -> Option<ComponentValue<'_>> {
        match component.kind {
            ComponentKind::Schemas => self
                .schemas
                .as_ref()?
                .get(&component.name)
                .map(ComponentValue::Schema),
            ComponentKind::Responses => self
                .responses
                .as_ref()?
                .get(&component.name)
                .map(ComponentValue::Response),
            ComponentKind::Parameters => self
                .parameters
                .as_ref()?
                .get(&component.name)
                .map(ComponentValue::Parameter),
            ComponentKind::Examples => self
                .examples
                .as_ref()?
                .get(&component.name)
                .map(ComponentValue::Example),
            ComponentKind::RequestBodies => self
                .request_bodies
                .as_ref()?
                .get(&component.name)
                .map(ComponentValue::RequestBody),
            ComponentKind::Headers => self
                .headers
                .as_ref()?
                .get(&component.name)
                .map(ComponentValue::Header),
            ComponentKind::SecuritySchemes => self
                .security_schemes
                .as_ref()?
                .get(&component.name)
                .map(ComponentValue::SecurityScheme),
            ComponentKind::Links => self
                .links
                .as_ref()?
                .get(&component.name)
                .map(ComponentValue::Link),
            ComponentKind::Callbacks => self
                .callbacks
                .as_ref()?
                .get(&component.name)
                .map(ComponentValue::Callback),
            ComponentKind::PathItems => self
                .path_items
                .as_ref()?
                .get(&component.name)
                .map(ComponentValue::PathItem),
        }
    }

    /// Rewrite every reference target inside these components through
    /// `renames`, and every security requirement key whose scheme is renamed.
    pub fn rename_refs(&mut self, renames: &BTreeMap<ComponentRef, String>) {
        write::components_node(&mut Rename { renames }, self);
    }

    /// Copy components from `source` under the names `renames` gives them, or
    /// under their own names when the map has no entry.
    ///
    /// A component whose target name is already present is left unchanged.
    /// References inside each copied component are rewritten through
    /// `renames`. Returns the components actually inserted, keyed by their new
    /// names.
    pub fn copy_from(
        &mut self,
        source: &Components,
        which: impl IntoIterator<Item = ComponentRef>,
        renames: &BTreeMap<ComponentRef, String>,
    ) -> Result<BTreeSet<ComponentRef>, RefError> {
        let mut copied = BTreeSet::new();
        for component in which {
            let Some(value) = source.get(&component) else {
                return Err(RefError::Missing(component));
            };
            let target = ComponentRef::new(
                component.kind,
                renames
                    .get(&component)
                    .cloned()
                    .unwrap_or_else(|| component.name.clone()),
            );
            if self.get(&target).is_some() {
                continue;
            }
            self.insert_component(&target, value);
            self.walk_component_mut(&target, &mut Rename { renames });
            copied.insert(target);
        }
        Ok(copied)
    }

    fn insert_component(&mut self, target: &ComponentRef, value: ComponentValue<'_>) {
        match value {
            ComponentValue::Schema(value) => {
                self.schemas
                    .get_or_insert_default()
                    .insert(target.name.clone(), value.clone());
            }
            ComponentValue::Response(value) => {
                self.responses
                    .get_or_insert_default()
                    .insert(target.name.clone(), value.clone());
            }
            ComponentValue::Parameter(value) => {
                self.parameters
                    .get_or_insert_default()
                    .insert(target.name.clone(), value.clone());
            }
            ComponentValue::Example(value) => {
                self.examples
                    .get_or_insert_default()
                    .insert(target.name.clone(), value.clone());
            }
            ComponentValue::RequestBody(value) => {
                self.request_bodies
                    .get_or_insert_default()
                    .insert(target.name.clone(), value.clone());
            }
            ComponentValue::Header(value) => {
                self.headers
                    .get_or_insert_default()
                    .insert(target.name.clone(), value.clone());
            }
            ComponentValue::SecurityScheme(value) => {
                self.security_schemes
                    .get_or_insert_default()
                    .insert(target.name.clone(), value.clone());
            }
            ComponentValue::Link(value) => {
                self.links
                    .get_or_insert_default()
                    .insert(target.name.clone(), value.clone());
            }
            ComponentValue::Callback(value) => {
                self.callbacks
                    .get_or_insert_default()
                    .insert(target.name.clone(), value.clone());
            }
            ComponentValue::PathItem(value) => {
                self.path_items
                    .get_or_insert_default()
                    .insert(target.name.clone(), value.clone());
            }
        }
    }
}

/// Split `#/components/{kind}/{name}[/tail...]` into the component and the
/// raw tail after the name token. The name token is RFC 6901 decoded; the tail
/// is returned as written, so a rename keeps a deeper pointer's suffix.
fn split_component_pointer(target: &str) -> Option<(ComponentRef, Option<&str>)> {
    let rest = target.strip_prefix("#/components/")?;
    let mut parts = rest.splitn(3, '/');
    let kind = ComponentKind::from_key(parts.next()?)?;
    let name_token = parts.next()?;
    let tail = parts.next();
    let name = json_pointer::tokens(&format!("/{name_token}")).ok()?;
    let [name] = <[String; 1]>::try_from(name).ok()?;
    Some((ComponentRef { kind, name }, tail))
}

/// A `discriminator.mapping` value with no pointer or file part is a schema
/// component name.
pub(crate) fn is_bare_name(target: &str) -> bool {
    !target.contains('#') && !target.contains('/')
}

/// Rewrites reference targets and security requirement keys through a map.
struct Rename<'a> {
    renames: &'a BTreeMap<ComponentRef, String>,
}

impl VisitMut for Rename<'_> {
    fn reference(&mut self, site: RefSiteMut<'_>) {
        if site.dynamic {
            return;
        }
        if site.mapping && is_bare_name(site.target) {
            let component = ComponentRef::new(ComponentKind::Schemas, site.target.as_str());
            if let Some(new_name) = self.renames.get(&component) {
                if *new_name != component.name {
                    *site.target = new_name.clone();
                }
            }
            return;
        }
        let Some((mut component, tail)) = split_component_pointer(site.target) else {
            return;
        };
        let Some(new_name) = self.renames.get(&component) else {
            return;
        };
        if *new_name == component.name {
            return;
        }
        component.name = new_name.clone();
        let mut pointer = component.to_pointer();
        if let Some(tail) = tail {
            pointer.push('/');
            pointer.push_str(tail);
        }
        *site.target = pointer;
    }

    fn security_requirement(&mut self, requirement: &mut crate::SecurityRequirement) {
        let mut rebuilt: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (name, scopes) in std::mem::take(&mut requirement.data) {
            let component = ComponentRef::new(ComponentKind::SecuritySchemes, &name);
            let new_name = self.renames.get(&component).cloned().unwrap_or(name);
            let merged = rebuilt.entry(new_name).or_default();
            for scope in scopes {
                if !merged.contains(&scope) {
                    merged.push(scope);
                }
            }
        }
        requirement.data = rebuilt;
    }
}

/// Collects direct references for [`Operation::refs`] and [`RefGraph`].
#[derive(Default)]
pub(crate) struct Collect(Vec<DirectRef>);

impl Collect {
    pub(crate) fn into_inner(self) -> Vec<DirectRef> {
        self.0
    }
}

impl Visit for Collect {
    fn reference(&mut self, site: RefSite<'_>) {
        self.0.push(DirectRef {
            kind: site.kind,
            target: site.target.to_owned(),
            dynamic: site.dynamic,
            mapping: site.mapping,
        });
    }

    fn security_requirement(&mut self, requirement: &crate::SecurityRequirement) {
        for name in requirement.data.keys() {
            self.0.push(DirectRef {
                kind: ComponentKind::SecuritySchemes,
                target: ComponentRef::new(ComponentKind::SecuritySchemes, name).to_pointer(),
                dynamic: false,
                mapping: false,
            });
        }
    }
}

/// Lists every `Schema` field so adding one fails to compile here until the
/// walk decides whether it holds references or schemas.
#[cfg(not(feature = "lossless-schema-keywords"))]
#[allow(dead_code)]
fn complete_schema(schema: &Schema) {
    let Schema {
        _type: _,
        _ref: _,
        format: _,
        nullable: _,
        description: _,
        content_media_type: _,
        content_encoding: _,
        content_schema: _,
        properties: _,
        pattern_properties: _,
        required: _,
        min_properties: _,
        max_properties: _,
        additional_properties: _,
        unevaluated_properties: _,
        property_names: _,
        all_of: _,
        any_of: _,
        one_of: _,
        not: _,
        if_schema: _,
        then_schema: _,
        else_schema: _,
        dependent_required: _,
        dependent_schemas: _,
        dependencies: _,
        defs: _,
        definitions: _,
        items: _,
        prefix_items: _,
        additional_items: _,
        unevaluated_items: _,
        contains: _,
        extras: _,
    } = schema;
}

/// Lists every `Schema` field so adding one fails to compile here until the
/// walk decides whether it holds references or schemas.
#[cfg(feature = "lossless-schema-keywords")]
#[allow(dead_code)]
fn complete_schema(schema: &Schema) {
    let Schema {
        _type: _,
        _ref: _,
        format: _,
        nullable: _,
        description: _,
        content_media_type: _,
        content_encoding: _,
        content_schema: _,
        schema_format: _,
        schema: _,
        external_docs: _,
        discriminator: _,
        properties: _,
        pattern_properties: _,
        required: _,
        min_properties: _,
        max_properties: _,
        additional_properties: _,
        unevaluated_properties: _,
        property_names: _,
        all_of: _,
        any_of: _,
        one_of: _,
        not: _,
        if_schema: _,
        then_schema: _,
        else_schema: _,
        dependent_required: _,
        dependent_schemas: _,
        dependencies: _,
        defs: _,
        definitions: _,
        items: _,
        prefix_items: _,
        additional_items: _,
        unevaluated_items: _,
        contains: _,
        extras: _,
    } = schema;
}
