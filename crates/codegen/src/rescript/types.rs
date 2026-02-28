//! `StdbTypes.res` generator — per-type submodule emission with topological ordering.
//!
//! Every type gets its own `module Foo = { type t = ... }` inside `StdbTypes.res`.
//! Three emission patterns based on SCC analysis:
//!
//! - **Pattern 1 (Standalone):** `module Foo = { type t = { ... } }`
//! - **Pattern 2 (Self-recursive):** `module Foo = { type rec t = { ... } }`
//! - **Pattern 3 (Mutual recursion):** Private `module Recursive_N = { type rec a = ... and b = ... }`
//!   with public `module A = { type t = Recursive_N.a }` and `module B = { type t = Recursive_N.b }`.

use super::helpers::{
    render_plain_enum, render_record_type_kw, render_sum_type, rescript_module_name, rescript_type_name, TypeRefStyle,
};
use super::templates::{AutoGenHeaderRes, ModuleTypeAliasRes, ModuleWrapperRes, TypesPostambleRes, TypesPreambleRes};
use super::topo::{topological_groups, TypeGroup};
use crate::util::{iter_types, type_ref_name};
use crate::OutputFile;

use spacetimedb_schema::def::ModuleDef;
use spacetimedb_schema::type_for_generate::AlgebraicTypeDef;

use std::fmt::Write;

pub fn generate_types_file(module: &ModuleDef) -> OutputFile {
    let mut code = String::new();

    // Preamble: auto-gen header + opaque SDK types.
    write!(code, "{}", AutoGenHeaderRes).unwrap();
    write!(code, "\n").unwrap();
    write!(code, "{}", TypesPreambleRes).unwrap();

    // Collect type refs for topological sort.
    let types: Vec<_> = iter_types(module).collect();
    if types.is_empty() {
        write!(code, "{}", TypesPostambleRes).unwrap();
        return OutputFile {
            filename: "StdbTypes.res".to_string(),
            code,
        };
    }

    let type_refs: Vec<_> = types.iter().map(|t| t.ty).collect();
    let groups = topological_groups(module, &type_refs);

    let mut recursive_group_id: usize = 0;

    for group in &groups {
        match group {
            TypeGroup::Standalone(r) => emit_standalone(module, &mut code, *r),
            TypeGroup::SelfRecursive(r) => emit_self_recursive(module, &mut code, *r),
            TypeGroup::MutuallyRecursive(refs) => {
                recursive_group_id += 1;
                emit_mutual_recursive(module, &mut code, refs, recursive_group_id);
            }
        }
    }

    // Postamble.
    write!(code, "{}", TypesPostambleRes).unwrap();

    OutputFile {
        filename: "StdbTypes.res".to_string(),
        code,
    }
}

// ---------------------------------------------------------------------------
// Pattern 1: Standalone type → `module Foo = { type t = ... }`
// ---------------------------------------------------------------------------

fn emit_standalone(module: &ModuleDef, code: &mut String, r: spacetimedb_lib::sats::AlgebraicTypeRef) {
    let pascal = type_ref_name(module, r);
    let mod_name = rescript_module_name(&pascal);
    let typespace = module.typespace_for_generate();

    let type_decl = match &typespace[r] {
        AlgebraicTypeDef::Product(product) => {
            render_record_type_kw(module, "type", "t", &product.elements, TypeRefStyle::InTypesFile)
        }
        AlgebraicTypeDef::Sum(sum) => render_sum_type(module, "type", "t", &sum.variants, TypeRefStyle::InTypesFile),
        AlgebraicTypeDef::PlainEnum(plain_enum) => render_plain_enum("type", "t", &plain_enum.variants),
    };

    let wrapper = ModuleWrapperRes {
        name: &mod_name,
        content: type_decl.trim_end(),
    };
    write!(code, "{wrapper}\n").unwrap();
}

// ---------------------------------------------------------------------------
// Pattern 2: Self-recursive type → `module Foo = { type rec t = ... }`
// ---------------------------------------------------------------------------

fn emit_self_recursive(module: &ModuleDef, code: &mut String, r: spacetimedb_lib::sats::AlgebraicTypeRef) {
    let pascal = type_ref_name(module, r);
    let mod_name = rescript_module_name(&pascal);
    let typespace = module.typespace_for_generate();

    let type_decl = match &typespace[r] {
        AlgebraicTypeDef::Product(product) => {
            render_record_type_kw(module, "type rec", "t", &product.elements, TypeRefStyle::InTypesFile)
        }
        AlgebraicTypeDef::Sum(sum) => {
            render_sum_type(module, "type rec", "t", &sum.variants, TypeRefStyle::InTypesFile)
        }
        AlgebraicTypeDef::PlainEnum(_) => {
            unreachable!("PlainEnum marked as self-recursive");
        }
    };

    let wrapper = ModuleWrapperRes {
        name: &mod_name,
        content: type_decl.trim_end(),
    };
    write!(code, "{wrapper}\n").unwrap();
}

// ---------------------------------------------------------------------------
// Pattern 3: Mutually recursive types
// ---------------------------------------------------------------------------

fn emit_mutual_recursive(
    module: &ModuleDef,
    code: &mut String,
    refs: &[spacetimedb_lib::sats::AlgebraicTypeRef],
    group_id: usize,
) {
    let typespace = module.typespace_for_generate();
    let group_module = format!("Recursive_{group_id}");

    // Build inner content: `type rec a = ... and b = ...`
    let mut inner = String::new();
    for (i, &r) in refs.iter().enumerate() {
        let pascal = type_ref_name(module, r);
        let type_alias = rescript_type_name(pascal.clone());
        let keyword = if i == 0 { "type rec" } else { "and" };

        let type_decl = match &typespace[r] {
            AlgebraicTypeDef::Product(product) => render_record_type_kw(
                module,
                keyword,
                &type_alias,
                &product.elements,
                TypeRefStyle::InRecursiveGroup,
            ),
            AlgebraicTypeDef::Sum(sum) => render_sum_type(
                module,
                keyword,
                &type_alias,
                &sum.variants,
                TypeRefStyle::InRecursiveGroup,
            ),
            AlgebraicTypeDef::PlainEnum(_) => {
                unreachable!("PlainEnum in mutual recursion group");
            }
        };

        inner.push_str(&type_decl);
    }

    // Private recursion group module.
    writeln!(
        code,
        "// Private recursion group — consumers use the public modules below"
    )
    .unwrap();
    let wrapper = ModuleWrapperRes {
        name: &group_module,
        content: inner.trim_end(),
    };
    write!(code, "{wrapper}\n").unwrap();

    // Public alias modules.
    for &r in refs {
        let pascal = type_ref_name(module, r);
        let mod_name = rescript_module_name(&pascal);
        let type_alias = rescript_type_name(pascal);
        let alias = ModuleTypeAliasRes {
            name: &mod_name,
            group_module: &group_module,
            type_alias: &type_alias,
        };
        write!(code, "{alias}").unwrap();
    }
    writeln!(code).unwrap();
}
