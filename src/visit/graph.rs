use std::collections::{BTreeMap, BTreeSet};

use super::{is_bare_name, Collect, ComponentKind, ComponentRef, DirectRef, RefError};
use crate::Components;

/// The component reference graph of one `components` object.
///
/// Edges are component to component: the components a component references
/// directly, through any position of its subtree. A component whose own refs
/// resolve but which reaches one that does not is in [`RefGraph::broken`] with
/// the same error, so closure and root checks can fail early.
///
/// Build it once with [`Components::ref_graph`] or
/// [`OpenAPIV3::ref_graph`](crate::OpenAPIV3::ref_graph); every question below
/// then walks only the edges it needs.
#[derive(Debug, Clone, Default)]
pub struct RefGraph {
    edges: BTreeMap<ComponentRef, Vec<ComponentRef>>,
    parents: BTreeMap<ComponentRef, Vec<ComponentRef>>,
    broken: BTreeMap<ComponentRef, RefError>,
}

impl RefGraph {
    /// The direct references of every component, including components that
    /// are broken themselves.
    pub fn edges(&self) -> &BTreeMap<ComponentRef, Vec<ComponentRef>> {
        &self.edges
    }

    /// Components whose closure contains an unresolvable reference, with the
    /// first such error in walk order.
    pub fn broken(&self) -> &BTreeMap<ComponentRef, RefError> {
        &self.broken
    }

    /// Resolve the references a node writes itself to the components that
    /// exist, rejecting dynamic, external, wrong-kind, missing and broken
    /// targets.
    pub fn roots(
        &self,
        targets: impl IntoIterator<Item = DirectRef>,
    ) -> Result<BTreeSet<ComponentRef>, RefError> {
        let mut roots = BTreeSet::new();
        for target in targets {
            let root = self.resolve(&target)?;
            if let Some(error) = self.broken.get(&root) {
                return Err(error.clone());
            }
            roots.insert(root);
        }
        Ok(roots)
    }

    /// Everything the roots reach, each component once. Cycles terminate. A
    /// missing, dynamic or broken root fails with the first such error.
    pub fn closure(
        &self,
        roots: impl IntoIterator<Item = ComponentRef>,
    ) -> Result<BTreeSet<ComponentRef>, RefError> {
        let mut reached = BTreeSet::new();
        let mut queue = Vec::new();
        for root in roots {
            if !self.edges.contains_key(&root) {
                return Err(RefError::Missing(root));
            }
            if let Some(error) = self.broken.get(&root) {
                return Err(error.clone());
            }
            queue.push(root);
        }
        while let Some(key) = queue.pop() {
            if !reached.insert(key.clone()) {
                continue;
            }
            for child in self.edges.get(&key).into_iter().flatten() {
                if !reached.contains(child) {
                    queue.push(child.clone());
                }
            }
        }
        Ok(reached)
    }

    /// The components in `within` that are `changed` or reach one of them
    /// through edges that stay inside `within`, including cycles.
    pub fn dependents(
        &self,
        changed: &BTreeSet<ComponentRef>,
        within: &BTreeSet<ComponentRef>,
    ) -> BTreeSet<ComponentRef> {
        let mut found: BTreeSet<ComponentRef> = changed
            .iter()
            .filter(|component| within.contains(*component))
            .cloned()
            .collect();
        let mut queue: Vec<ComponentRef> = found.iter().cloned().collect();
        while let Some(key) = queue.pop() {
            for parent in self.parents.get(&key).into_iter().flatten() {
                if within.contains(parent) && found.insert(parent.clone()) {
                    queue.push(parent.clone());
                }
            }
        }
        found
    }

    fn resolve(&self, target: &DirectRef) -> Result<ComponentRef, RefError> {
        resolve(target, |component| self.edges.contains_key(component))
    }
}

impl Components {
    /// Build the reference graph of this `components` object. A component
    /// that references another that is not in these components is broken, and
    /// so is every component that reaches it.
    pub fn ref_graph(&self) -> RefGraph {
        let mut edges = BTreeMap::new();
        let mut broken = BTreeMap::new();
        for key in component_refs(self) {
            let mut collect = Collect::default();
            self.walk_component(&key, &mut collect);
            let mut children = Vec::new();
            for direct in collect.into_inner() {
                match resolve(&direct, |component| self.get(component).is_some()) {
                    Ok(child) => children.push(child),
                    Err(error) => {
                        broken.entry(key.clone()).or_insert(error);
                    }
                }
            }
            edges.insert(key, children);
        }

        let mut parents: BTreeMap<ComponentRef, Vec<ComponentRef>> = BTreeMap::new();
        for (parent, children) in &edges {
            for child in children {
                parents
                    .entry(child.clone())
                    .or_default()
                    .push(parent.clone());
            }
        }
        let mut queue: Vec<ComponentRef> = broken.keys().cloned().collect();
        while let Some(key) = queue.pop() {
            let error = broken[&key].clone();
            for parent in parents.get(&key).into_iter().flatten() {
                if !broken.contains_key(parent) {
                    broken.insert(parent.clone(), error.clone());
                    queue.push(parent.clone());
                }
            }
        }

        RefGraph {
            edges,
            parents,
            broken,
        }
    }
}

/// Resolve one direct reference against the component names `exists` accepts.
///
/// A `discriminator.mapping` value with no `#` or `/` is a bare schema name
/// and resolves under `components.schemas`. Other targets must be exact
/// `#/components/{kind}/{name}` pointers.
fn resolve(
    target: &DirectRef,
    exists: impl Fn(&ComponentRef) -> bool,
) -> Result<ComponentRef, RefError> {
    if target.dynamic {
        return Err(RefError::Dynamic);
    }
    let parsed = if target.mapping && is_bare_name(&target.target) {
        ComponentRef::new(ComponentKind::Schemas, &target.target)
    } else {
        ComponentRef::parse(&target.target)?
    };
    if parsed.kind != target.kind {
        return Err(RefError::WrongKind {
            expected: target.kind,
            found: parsed.kind,
        });
    }
    if !exists(&parsed) {
        return Err(RefError::Missing(parsed));
    }
    Ok(parsed)
}

/// Every component name in `components`, in [`ComponentKind::ALL`] order.
fn component_refs(components: &Components) -> Vec<ComponentRef> {
    let mut refs = Vec::new();
    for kind in ComponentKind::ALL {
        let names: Vec<String> = match kind {
            ComponentKind::Schemas => components
                .schemas
                .iter()
                .flat_map(BTreeMap::keys)
                .cloned()
                .collect(),
            ComponentKind::Responses => components
                .responses
                .iter()
                .flat_map(BTreeMap::keys)
                .cloned()
                .collect(),
            ComponentKind::Parameters => components
                .parameters
                .iter()
                .flat_map(BTreeMap::keys)
                .cloned()
                .collect(),
            ComponentKind::Examples => components
                .examples
                .iter()
                .flat_map(BTreeMap::keys)
                .cloned()
                .collect(),
            ComponentKind::RequestBodies => components
                .request_bodies
                .iter()
                .flat_map(BTreeMap::keys)
                .cloned()
                .collect(),
            ComponentKind::Headers => components
                .headers
                .iter()
                .flat_map(BTreeMap::keys)
                .cloned()
                .collect(),
            ComponentKind::SecuritySchemes => components
                .security_schemes
                .iter()
                .flat_map(BTreeMap::keys)
                .cloned()
                .collect(),
            ComponentKind::Links => components
                .links
                .iter()
                .flat_map(BTreeMap::keys)
                .cloned()
                .collect(),
            ComponentKind::Callbacks => components
                .callbacks
                .iter()
                .flat_map(BTreeMap::keys)
                .cloned()
                .collect(),
            ComponentKind::PathItems => components
                .path_items
                .iter()
                .flat_map(BTreeMap::keys)
                .cloned()
                .collect(),
        };
        refs.extend(names.into_iter().map(|name| ComponentRef::new(kind, name)));
    }
    refs
}
