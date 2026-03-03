//! Topological sort with SCC (Strongly Connected Components) for ReScript type emission.
//!
//! Layer-A shared implementation lives in `sigma_rescript_codegen::topo`.
//! This module adapts SpacetimeDB `ModuleDef` graph data into that generic interface.

use spacetimedb_lib::sats::AlgebraicTypeRef;
use spacetimedb_schema::def::ModuleDef;
use spacetimedb_schema::type_for_generate::AlgebraicTypeDef;

use sigma_rescript_codegen::topo::{self as shared_topo, TypeGraph};

pub use sigma_rescript_codegen::topo::TypeGroup;

/// Run Tarjan's SCC on the type graph, then return groups in topological order
/// (dependencies before dependents — leaves first).
///
/// Only considers types that appear in `type_refs` (the set of user-defined types
/// that codegen should emit). Internal/anonymous types are excluded.
pub fn topological_groups(module: &ModuleDef, type_refs: &[AlgebraicTypeRef]) -> Vec<TypeGroup<AlgebraicTypeRef>> {
    let graph = ModuleTypeGraph {
        module,
        type_refs,
        ref_set: type_refs.iter().copied().collect(),
    };
    shared_topo::topological_groups(&graph)
}

struct ModuleTypeGraph<'a> {
    module: &'a ModuleDef,
    type_refs: &'a [AlgebraicTypeRef],
    ref_set: std::collections::HashSet<AlgebraicTypeRef>,
}

impl TypeGraph for ModuleTypeGraph<'_> {
    type Id = AlgebraicTypeRef;

    fn nodes(&self) -> Vec<Self::Id> {
        self.type_refs.to_vec()
    }

    fn dependencies(&self, node: Self::Id) -> Vec<Self::Id> {
        let def = &self.module.typespace_for_generate()[node];
        extract_refs(def)
            .into_iter()
            .filter(|dep| self.ref_set.contains(dep))
            .collect()
    }
}

/// Extract type references from an `AlgebraicTypeDef`.
/// Uses the public `AlgebraicTypeUse::for_each_ref` method to walk the type tree.
fn extract_refs(def: &AlgebraicTypeDef) -> Vec<AlgebraicTypeRef> {
    let mut refs = std::collections::HashSet::new();
    match def {
        AlgebraicTypeDef::Product(prod) => {
            for (_, use_) in &prod.elements {
                use_.for_each_ref(|r| {
                    refs.insert(r);
                });
            }
        }
        AlgebraicTypeDef::Sum(sum) => {
            for (_, use_) in &sum.variants {
                use_.for_each_ref(|r| {
                    refs.insert(r);
                });
            }
        }
        AlgebraicTypeDef::PlainEnum(_) => {}
    }
    refs.into_iter().collect()
}
