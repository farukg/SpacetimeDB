//! Shared ReScript codegen helpers: type rendering, name munging, record/sum emission.
//!
//! `TypeRefStyle` replaces the old `in_types_file: bool` parameter with a clear enum
//! that handles the three possible reference contexts.

use crate::code_indenter::Indenter;
use crate::util::type_ref_name;

use convert_case::{Case, Casing};
use spacetimedb_lib::sats::layout::PrimitiveType;
use spacetimedb_schema::def::ModuleDef;
use spacetimedb_schema::identifier::Identifier;
use spacetimedb_schema::type_for_generate::AlgebraicTypeUse;
use std::ops::Deref;

/// How to render a reference to another type (`AlgebraicTypeUse::Ref`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeRefStyle {
    /// Inside `StdbTypes.res`, sibling module reference: `AccountId.t`
    InTypesFile,
    /// Inside a `module Recursive_N = { ... }` block: bare name (e.g., `expr`)
    InRecursiveGroup,
    /// From table/reducer files (outside StdbTypes.res): `StdbTypes.AccountId.t`
    External,
}

// ---------------------------------------------------------------------------
// Name munging
// ---------------------------------------------------------------------------

/// Convert a PascalCase type name to a ReScript module name (PascalCase, escaping reserved words).
pub fn rescript_module_name(name: &str) -> String {
    // ReScript module names are PascalCase — the input from `type_ref_name()` is already Pascal.
    // We only need to handle the rare case where a name collides with a built-in.
    match name {
        "Array" => "Array_".to_string(),
        "Option" => "Option_".to_string(),
        "Result" => "Result_".to_string(),
        "String" => "String_".to_string(),
        _ => name.to_string(),
    }
}

pub fn rescript_type_name(type_name_pascal: String) -> String {
    rescript_field_name(type_name_pascal.to_case(Case::Camel))
}

pub fn rescript_field_name(name: String) -> String {
    match name.as_str() {
        "and" | "assert" | "constraint" | "exception" | "external" | "for" | "if" | "in" | "include" | "let"
        | "module" | "mutable" | "open" | "private" | "rec" | "switch" | "type" | "when" | "while" => {
            format!("{name}_")
        }
        _ => name,
    }
}

pub fn rescript_constructor_name(name: &str) -> String {
    // SpacetimeDB's module-def validation layer applies camelCase conversion
    // to enum variant names before codegen receives them:
    //   Rust `UG` → `ug`, `AG` → `ag`, `OHG` → `ohg`, `KG` → `kg`
    //   Rust `GmbH` → `gmbH`, `GbR` → `gbR`
    //
    // Heuristic to recover acronyms: short (2-3 char) all-lowercase ASCII-alpha
    // names are almost certainly acronyms. Uppercase them entirely:
    //   ug → UG, ag → AG, ohg → OHG, kg → KG
    //
    // Mixed-case short names like `gmbH` and `gbR` are NOT all-lowercase,
    // so they fall through to `to_case(Case::Pascal)` → GmbH, GbR. ✓
    if name.len() >= 2 && name.len() <= 3 && name.chars().all(|c| c.is_ascii_lowercase()) {
        return name.to_ascii_uppercase();
    }

    // If the name is already PascalCase (no underscores, starts with uppercase),
    // use as-is to preserve mixed casing (e.g. GmbH, GbR).
    let first_char = name.chars().next();
    if first_char.is_some_and(|c| c.is_ascii_uppercase()) && !name.contains('_') {
        return name.to_string();
    }

    let pascal = name.to_case(Case::Pascal);
    if pascal.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("V{pascal}")
    } else {
        pascal
    }
}

pub fn table_module_name(table_name: &Identifier) -> String {
    format!("Stdb{}Table", table_name.deref().to_case(Case::Pascal))
}

pub fn reducer_module_name(reducer_name: &spacetimedb_schema::reducer_name::ReducerName) -> String {
    format!("Stdb{}Reducer", reducer_name.deref().to_case(Case::Pascal))
}

pub fn procedure_module_name(procedure_name: &Identifier) -> String {
    format!("Stdb{}Procedure", procedure_name.deref().to_case(Case::Pascal))
}

// ---------------------------------------------------------------------------
// Type rendering
// ---------------------------------------------------------------------------

/// Write a ReScript type reference for `AlgebraicTypeUse`.
///
/// `style` controls how `Ref` types are qualified.
pub fn write_res_type(module: &ModuleDef, out: &mut Indenter, ty: &AlgebraicTypeUse, style: TypeRefStyle) {
    match ty {
        AlgebraicTypeUse::Unit => {
            write!(out, "unit");
        }
        AlgebraicTypeUse::Never => {
            write!(out, "unit");
        }
        AlgebraicTypeUse::Identity | AlgebraicTypeUse::ConnectionId | AlgebraicTypeUse::Uuid => {
            write!(out, "string");
        }
        AlgebraicTypeUse::Timestamp => match style {
            TypeRefStyle::InTypesFile | TypeRefStyle::InRecursiveGroup => write!(out, "timestamp"),
            TypeRefStyle::External => write!(out, "StdbTypes.timestamp"),
        },
        AlgebraicTypeUse::TimeDuration => match style {
            TypeRefStyle::InTypesFile | TypeRefStyle::InRecursiveGroup => write!(out, "timeDuration"),
            TypeRefStyle::External => write!(out, "StdbTypes.timeDuration"),
        },
        AlgebraicTypeUse::ScheduleAt => match style {
            TypeRefStyle::InTypesFile | TypeRefStyle::InRecursiveGroup => write!(out, "scheduleAt"),
            TypeRefStyle::External => write!(out, "StdbTypes.scheduleAt"),
        },
        AlgebraicTypeUse::Option(inner) => {
            write!(out, "option<");
            write_res_type(module, out, inner, style);
            write!(out, ">");
        }
        AlgebraicTypeUse::Result { ok_ty, err_ty } => {
            write!(out, "result<");
            write_res_type(module, out, ok_ty, style);
            write!(out, ", ");
            write_res_type(module, out, err_ty, style);
            write!(out, ">");
        }
        AlgebraicTypeUse::Primitive(prim) => match prim {
            PrimitiveType::Bool => write!(out, "bool"),
            PrimitiveType::I8
            | PrimitiveType::U8
            | PrimitiveType::I16
            | PrimitiveType::U16
            | PrimitiveType::I32
            | PrimitiveType::U32 => write!(out, "int"),
            PrimitiveType::I64
            | PrimitiveType::U64
            | PrimitiveType::I128
            | PrimitiveType::U128
            | PrimitiveType::I256
            | PrimitiveType::U256 => write!(out, "bigint"),
            PrimitiveType::F32 | PrimitiveType::F64 => write!(out, "float"),
        },
        AlgebraicTypeUse::String => {
            write!(out, "string");
        }
        AlgebraicTypeUse::Array(inner) => {
            write!(out, "array<");
            write_res_type(module, out, inner, style);
            write!(out, ">");
        }
        AlgebraicTypeUse::Ref(reference) => {
            let pascal_name = type_ref_name(module, *reference);
            let module_name = rescript_module_name(&pascal_name);
            match style {
                TypeRefStyle::InTypesFile => {
                    // Sibling module inside StdbTypes.res: AccountId.t
                    write!(out, "{module_name}.t");
                }
                TypeRefStyle::InRecursiveGroup => {
                    // Inside a recursive group: use the lowercased type alias name
                    let type_alias = rescript_type_name(pascal_name);
                    write!(out, "{type_alias}");
                }
                TypeRefStyle::External => {
                    // From outside StdbTypes.res: StdbTypes.AccountId.t
                    write!(out, "StdbTypes.{module_name}.t");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Record / Sum type emission
// ---------------------------------------------------------------------------

/// Write `type <name> = { @as("raw") camel: T, ... }` (no `rec`).
pub fn write_record_type(
    module: &ModuleDef,
    out: &mut Indenter,
    name: &str,
    elements: &[(Identifier, AlgebraicTypeUse)],
    style: TypeRefStyle,
) {
    write_record_type_kw(module, out, "type", name, elements, style);
}

/// Write `<keyword> <name> = { ... }` where keyword is `type`, `type rec`, or `and`.
pub fn write_record_type_kw(
    module: &ModuleDef,
    out: &mut Indenter,
    keyword: &str,
    name: &str,
    elements: &[(Identifier, AlgebraicTypeUse)],
    style: TypeRefStyle,
) {
    if elements.is_empty() {
        writeln!(out, "{keyword} {name} = unit");
        writeln!(out, "");
        return;
    }

    writeln!(out, "{keyword} {name} = {{");
    out.indent(1);
    for (field, ty) in elements {
        let raw = field.deref();
        let camel = rescript_field_name(raw.to_case(Case::Camel));
        write!(out, "@as(\"{raw}\") ");
        write!(out, "{camel}: ");
        write_res_type(module, out, ty, style);
        writeln!(out, ",");
    }
    out.dedent(1);
    writeln!(out, "}}");
    writeln!(out, "");
}

/// Write a sum type with `@tag("tag")` discrimination.
pub fn write_sum_type(
    module: &ModuleDef,
    out: &mut Indenter,
    keyword: &str,
    name: &str,
    variants: &[(Identifier, AlgebraicTypeUse)],
    style: TypeRefStyle,
) {
    writeln!(out, "@tag(\"tag\")");
    writeln!(out, "{keyword} {name} =");
    out.indent(1);
    for (variant_name, variant_type) in variants {
        let constructor = rescript_constructor_name(variant_name.deref());
        if matches!(variant_type, AlgebraicTypeUse::Unit) {
            writeln!(out, "| {constructor}");
        } else {
            write!(out, "| {constructor}(");
            write_res_type(module, out, variant_type, style);
            writeln!(out, ")");
        }
    }
    out.dedent(1);
    writeln!(out, "");
}

/// Write a plain enum (all-unit variants, no `@tag` needed — compiles to strings).
pub fn write_plain_enum(out: &mut Indenter, keyword: &str, name: &str, variants: &[Identifier]) {
    writeln!(out, "{keyword} {name} =");
    out.indent(1);
    for variant in variants {
        let constructor = rescript_constructor_name(variant.deref());
        writeln!(out, "| {constructor}");
    }
    out.dedent(1);
    writeln!(out, "");
}
