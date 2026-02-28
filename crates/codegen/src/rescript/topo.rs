//! Topological sort with SCC (Strongly Connected Components) for ReScript type emission.
//!
//! Implements Tarjan's SCC algorithm without requiring `petgraph` (not a dep of the codegen crate).
//! Operates on `AlgebraicTypeRef` keys from the module's typespace.
//!
//! The result classifies every type into one of three patterns:
//! - **Pattern 1 (Standalone):** No self-reference, SCC size 1 → `module Foo = { type t = ... }`
//! - **Pattern 2 (Self-recursive):** Self-referencing, SCC size 1 → `module Foo = { type rec t = ... }`
//! - **Pattern 3 (Mutual recursion):** SCC size > 1 → private `Recursive_N` + public alias modules

use spacetimedb_lib::sats::AlgebraicTypeRef;
use spacetimedb_schema::def::ModuleDef;
use spacetimedb_schema::type_for_generate::AlgebraicTypeDef;
use std::collections::HashMap;

/// A group of types that form a strongly connected component.
#[derive(Debug, Clone)]
pub enum TypeGroup {
    /// Pattern 1: standalone type (no recursion).
    Standalone(AlgebraicTypeRef),
    /// Pattern 2: self-recursive type (references itself, but no mutual recursion).
    SelfRecursive(AlgebraicTypeRef),
    /// Pattern 3: mutually recursive types (cycle of 2+ types).
    MutuallyRecursive(Vec<AlgebraicTypeRef>),
}

/// Run Tarjan's SCC on the type graph, then return groups in topological order
/// (dependencies before dependents — leaves first).
///
/// Only considers types that appear in `type_refs` (the set of user-defined types
/// that codegen should emit). Internal/anonymous types are excluded.
pub fn topological_groups(module: &ModuleDef, type_refs: &[AlgebraicTypeRef]) -> Vec<TypeGroup> {
    let typespace = module.typespace_for_generate();

    // Build adjacency list: ref → set of refs it depends on (that are also in type_refs).
    let ref_set: std::collections::HashSet<AlgebraicTypeRef> = type_refs.iter().copied().collect();
    let mut adjacency: HashMap<AlgebraicTypeRef, Vec<AlgebraicTypeRef>> = HashMap::new();

    for &r in type_refs {
        let def = &typespace[r];
        let deps: Vec<AlgebraicTypeRef> = extract_refs(def)
            .into_iter()
            .filter(|dep| ref_set.contains(dep))
            .collect();
        adjacency.insert(r, deps);
    }

    // Run Tarjan's SCC.
    let sccs = tarjan_scc(type_refs, &adjacency);

    // Tarjan's returns SCCs in reverse topological order (sinks first).
    // We want dependencies-first (leaves first), which is already the right order
    // for Tarjan's standard output. But petgraph's tarjan_scc returns in reverse
    // post-order (roots first). Our implementation follows the standard algorithm
    // which pushes completed SCCs onto a stack → they come out leaves-first.
    //
    // Classify each SCC.
    sccs.into_iter()
        .map(|scc| {
            if scc.len() == 1 {
                let r = scc[0];
                if is_self_recursive(r, &adjacency) {
                    TypeGroup::SelfRecursive(r)
                } else {
                    TypeGroup::Standalone(r)
                }
            } else {
                TypeGroup::MutuallyRecursive(scc)
            }
        })
        .collect()
}

/// Extract type references from an `AlgebraicTypeDef`.
/// Uses the public `AlgebraicTypeUse::for_each_ref` method to walk the type tree.
fn extract_refs(def: &AlgebraicTypeDef) -> Vec<AlgebraicTypeRef> {
    let mut refs = std::collections::HashSet::new();
    match def {
        AlgebraicTypeDef::Product(prod) => {
            for (_, use_) in prod.elements.iter() {
                use_.for_each_ref(|r| {
                    refs.insert(r);
                });
            }
        }
        AlgebraicTypeDef::Sum(sum) => {
            for (_, use_) in sum.variants.iter() {
                use_.for_each_ref(|r| {
                    refs.insert(r);
                });
            }
        }
        AlgebraicTypeDef::PlainEnum(_) => {}
    }
    refs.into_iter().collect()
}

/// Check if a type directly references itself.
fn is_self_recursive(r: AlgebraicTypeRef, adjacency: &HashMap<AlgebraicTypeRef, Vec<AlgebraicTypeRef>>) -> bool {
    adjacency.get(&r).is_some_and(|deps| deps.contains(&r))
}

// ---------------------------------------------------------------------------
// Tarjan's SCC — textbook implementation.
// ---------------------------------------------------------------------------

struct TarjanState {
    index_counter: u32,
    stack: Vec<AlgebraicTypeRef>,
    on_stack: HashMap<AlgebraicTypeRef, bool>,
    index: HashMap<AlgebraicTypeRef, u32>,
    lowlink: HashMap<AlgebraicTypeRef, u32>,
    result: Vec<Vec<AlgebraicTypeRef>>,
}

/// Tarjan's SCC algorithm. Returns SCCs in topological order (leaves first).
fn tarjan_scc(
    nodes: &[AlgebraicTypeRef],
    adjacency: &HashMap<AlgebraicTypeRef, Vec<AlgebraicTypeRef>>,
) -> Vec<Vec<AlgebraicTypeRef>> {
    let mut state = TarjanState {
        index_counter: 0,
        stack: Vec::new(),
        on_stack: HashMap::new(),
        index: HashMap::new(),
        lowlink: HashMap::new(),
        result: Vec::new(),
    };

    for &node in nodes {
        if !state.index.contains_key(&node) {
            strongconnect(&mut state, node, adjacency);
        }
    }

    state.result
}

fn strongconnect(
    state: &mut TarjanState,
    v: AlgebraicTypeRef,
    adjacency: &HashMap<AlgebraicTypeRef, Vec<AlgebraicTypeRef>>,
) {
    state.index.insert(v, state.index_counter);
    state.lowlink.insert(v, state.index_counter);
    state.index_counter += 1;
    state.stack.push(v);
    state.on_stack.insert(v, true);

    // Visit successors.
    if let Some(successors) = adjacency.get(&v) {
        for &w in successors {
            if !state.index.contains_key(&w) {
                // w has not been visited; recurse.
                strongconnect(state, w, adjacency);
                let w_lowlink = state.lowlink[&w];
                let v_lowlink = state.lowlink.get_mut(&v).unwrap();
                if w_lowlink < *v_lowlink {
                    *v_lowlink = w_lowlink;
                }
            } else if state.on_stack.get(&w).copied().unwrap_or(false) {
                // w is on the stack → part of the current SCC.
                let w_index = state.index[&w];
                let v_lowlink = state.lowlink.get_mut(&v).unwrap();
                if w_index < *v_lowlink {
                    *v_lowlink = w_index;
                }
            }
        }
    }

    // If v is a root node, pop the SCC.
    if state.lowlink[&v] == state.index[&v] {
        let mut scc = Vec::new();
        loop {
            let w = state.stack.pop().unwrap();
            state.on_stack.insert(w, false);
            scc.push(w);
            if w == v {
                break;
            }
        }
        // Reverse so the root is first (stable ordering).
        scc.reverse();
        state.result.push(scc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ref_(n: u32) -> AlgebraicTypeRef {
        AlgebraicTypeRef(n)
    }

    #[test]
    fn standalone_types_in_order() {
        // A → B → C (linear chain, no cycles)
        let nodes = vec![ref_(0), ref_(1), ref_(2)];
        let mut adj = HashMap::new();
        adj.insert(ref_(0), vec![ref_(1)]);
        adj.insert(ref_(1), vec![ref_(2)]);
        adj.insert(ref_(2), vec![]);

        let sccs = tarjan_scc(&nodes, &adj);
        // Topological: C first, then B, then A
        assert_eq!(sccs.len(), 3);
        assert_eq!(sccs[0], vec![ref_(2)]);
        assert_eq!(sccs[1], vec![ref_(1)]);
        assert_eq!(sccs[2], vec![ref_(0)]);
    }

    #[test]
    fn self_recursive_detected() {
        let nodes = vec![ref_(0)];
        let mut adj = HashMap::new();
        adj.insert(ref_(0), vec![ref_(0)]);

        let sccs = tarjan_scc(&nodes, &adj);
        assert_eq!(sccs.len(), 1);
        assert_eq!(sccs[0], vec![ref_(0)]);
        assert!(is_self_recursive(ref_(0), &adj));
    }

    #[test]
    fn mutual_recursion_detected() {
        // A ↔ B
        let nodes = vec![ref_(0), ref_(1)];
        let mut adj = HashMap::new();
        adj.insert(ref_(0), vec![ref_(1)]);
        adj.insert(ref_(1), vec![ref_(0)]);

        let sccs = tarjan_scc(&nodes, &adj);
        assert_eq!(sccs.len(), 1);
        assert_eq!(sccs[0].len(), 2);
    }

    #[test]
    fn mixed_graph() {
        // C (leaf) ← A ↔ B
        let nodes = vec![ref_(0), ref_(1), ref_(2)];
        let mut adj = HashMap::new();
        adj.insert(ref_(0), vec![ref_(1), ref_(2)]); // A → B, A → C
        adj.insert(ref_(1), vec![ref_(0)]); // B → A (cycle A↔B)
        adj.insert(ref_(2), vec![]); // C (leaf)

        let sccs = tarjan_scc(&nodes, &adj);
        // C is a leaf → comes first; A↔B is one SCC → comes second
        assert_eq!(sccs.len(), 2);
        assert_eq!(sccs[0], vec![ref_(2)]); // C standalone
        assert_eq!(sccs[1].len(), 2); // A↔B mutual
    }
}
