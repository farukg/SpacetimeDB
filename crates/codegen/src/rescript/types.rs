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
    rescript_module_name, rescript_type_name, write_plain_enum, write_record_type_kw, write_sum_type, TypeRefStyle,
};
use super::topo::{topological_groups, TypeGroup};
use crate::code_indenter::{CodeIndenter, Indenter};
use crate::util::{iter_types, print_auto_generated_file_comment, type_ref_name};
use crate::OutputFile;

use spacetimedb_schema::def::ModuleDef;
use spacetimedb_schema::type_for_generate::AlgebraicTypeDef;

pub fn generate_types_file(module: &ModuleDef) -> OutputFile {
    let mut output = CodeIndenter::new(String::new(), super::INDENT);
    let out = &mut output;

    print_auto_generated_file_comment(out);
    writeln!(out, "");

    // ── Opaque SDK types (always emitted) ──
    emit_sdk_preamble(out);

    // Collect type refs for topological sort.
    let types: Vec<_> = iter_types(module).collect();
    if types.is_empty() {
        emit_postamble(out);
        return OutputFile {
            filename: "StdbTypes.res".to_string(),
            code: output.into_inner(),
        };
    }

    let type_refs: Vec<_> = types.iter().map(|t| t.ty).collect();
    let groups = topological_groups(module, &type_refs);

    // Reset counter for mutual recursion groups.
    let mut recursive_group_id: usize = 0;

    for group in &groups {
        match group {
            TypeGroup::Standalone(r) => emit_standalone(module, out, *r),
            TypeGroup::SelfRecursive(r) => emit_self_recursive(module, out, *r),
            TypeGroup::MutuallyRecursive(refs) => {
                recursive_group_id += 1;
                emit_mutual_recursive(module, out, refs, recursive_group_id);
            }
        }
    }

    emit_postamble(out);

    OutputFile {
        filename: "StdbTypes.res".to_string(),
        code: output.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Pattern 1: Standalone type → `module Foo = { type t = ... }`
// ---------------------------------------------------------------------------

fn emit_standalone(module: &ModuleDef, out: &mut Indenter, r: spacetimedb_lib::sats::AlgebraicTypeRef) {
    let pascal = type_ref_name(module, r);
    let mod_name = rescript_module_name(&pascal);
    let typespace = module.typespace_for_generate();

    writeln!(out, "module {mod_name} = {{");
    out.indent(1);

    match &typespace[r] {
        AlgebraicTypeDef::Product(product) => {
            write_record_type_kw(module, out, "type", "t", &product.elements, TypeRefStyle::InTypesFile);
        }
        AlgebraicTypeDef::Sum(sum) => {
            write_sum_type(module, out, "type", "t", &sum.variants, TypeRefStyle::InTypesFile);
        }
        AlgebraicTypeDef::PlainEnum(plain_enum) => {
            write_plain_enum(out, "type", "t", &plain_enum.variants);
        }
    }

    out.dedent(1);
    writeln!(out, "}}");
    writeln!(out, "");
}

// ---------------------------------------------------------------------------
// Pattern 2: Self-recursive type → `module Foo = { type rec t = ... }`
// ---------------------------------------------------------------------------

fn emit_self_recursive(module: &ModuleDef, out: &mut Indenter, r: spacetimedb_lib::sats::AlgebraicTypeRef) {
    let pascal = type_ref_name(module, r);
    let mod_name = rescript_module_name(&pascal);
    let typespace = module.typespace_for_generate();

    writeln!(out, "module {mod_name} = {{");
    out.indent(1);

    match &typespace[r] {
        AlgebraicTypeDef::Product(product) => {
            write_record_type_kw(
                module,
                out,
                "type rec",
                "t",
                &product.elements,
                TypeRefStyle::InTypesFile,
            );
        }
        AlgebraicTypeDef::Sum(sum) => {
            write_sum_type(module, out, "type rec", "t", &sum.variants, TypeRefStyle::InTypesFile);
        }
        AlgebraicTypeDef::PlainEnum(_) => {
            // PlainEnums cannot be self-recursive — this shouldn't happen.
            // Fall back to standalone emission.
            unreachable!("PlainEnum marked as self-recursive");
        }
    }

    out.dedent(1);
    writeln!(out, "}}");
    writeln!(out, "");
}

// ---------------------------------------------------------------------------
// Pattern 3: Mutually recursive types
//   module Recursive_N = { type rec a = ... and b = ... }
//   module A = { type t = Recursive_N.a }
//   module B = { type t = Recursive_N.b }
// ---------------------------------------------------------------------------

fn emit_mutual_recursive(
    module: &ModuleDef,
    out: &mut Indenter,
    refs: &[spacetimedb_lib::sats::AlgebraicTypeRef],
    group_id: usize,
) {
    let typespace = module.typespace_for_generate();
    let group_module = format!("Recursive_{group_id}");

    // Private recursion group module.
    writeln!(
        out,
        "// Private recursion group — consumers use the public modules below"
    );
    writeln!(out, "module {group_module} = {{");
    out.indent(1);

    for (i, &r) in refs.iter().enumerate() {
        let pascal = type_ref_name(module, r);
        let type_alias = rescript_type_name(pascal.clone());
        let keyword = if i == 0 { "type rec" } else { "and" };

        match &typespace[r] {
            AlgebraicTypeDef::Product(product) => {
                write_record_type_kw(
                    module,
                    out,
                    keyword,
                    &type_alias,
                    &product.elements,
                    TypeRefStyle::InRecursiveGroup,
                );
            }
            AlgebraicTypeDef::Sum(sum) => {
                write_sum_type(
                    module,
                    out,
                    keyword,
                    &type_alias,
                    &sum.variants,
                    TypeRefStyle::InRecursiveGroup,
                );
            }
            AlgebraicTypeDef::PlainEnum(_) => {
                unreachable!("PlainEnum in mutual recursion group");
            }
        }
    }

    out.dedent(1);
    writeln!(out, "}}");
    writeln!(out, "");

    // Public stable modules (1:1 name mapping preserved).
    for &r in refs {
        let pascal = type_ref_name(module, r);
        let mod_name = rescript_module_name(&pascal);
        let type_alias = rescript_type_name(pascal);
        writeln!(out, "module {mod_name} = {{ type t = {group_module}.{type_alias} }}");
    }
    writeln!(out, "");
}

// ---------------------------------------------------------------------------
// Preamble / postamble
// ---------------------------------------------------------------------------

fn emit_sdk_preamble(out: &mut Indenter) {
    writeln!(out, "// Opaque SDK types");
    writeln!(out, "type connection");
    writeln!(out, "type eventCtx");
    writeln!(out, "type reducers");
    writeln!(out, "");
    writeln!(out, "// Opaque SDK Timestamp — use toDate, toMillis, or toFloatMs");
    writeln!(out, "type timestamp");
    writeln!(out, "@send external toMillis: (timestamp) => bigint = \"toMillis\"");
    writeln!(out, "@send external toDate: (timestamp) => Date.t = \"toDate\"");
    writeln!(
        out,
        "let toFloatMs = (ts: timestamp): float => ts->toMillis->BigInt.toFloat"
    );
    writeln!(out, "");

    writeln!(out, "type timeDuration  // opaque — SDK TimeDuration class instance");
    writeln!(
        out,
        "@send external timeDurationToMicros: (timeDuration) => bigint = \"toMicros\""
    );
    writeln!(out, "");

    writeln!(out, "// ScheduleAt — SDK built-in tagged union");
    writeln!(out, "@tag(\"tag\")");
    writeln!(out, "type scheduleAt =");
    writeln!(out, "  | Interval({{value: timeDuration}})");
    writeln!(out, "  | Time({{value: timestamp}})");
    writeln!(out, "");
}

fn emit_postamble(out: &mut Indenter) {
    writeln!(out, "");
    writeln!(
        out,
        "// SDK connection builder — opaque, constructed via DbConnection.builder()"
    );
    writeln!(out, "type connectionBuilder");
}
