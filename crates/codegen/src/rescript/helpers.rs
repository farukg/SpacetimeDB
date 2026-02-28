//! Shared ReScript codegen helpers: type rendering, name munging, record/sum emission.
//!
//! `TypeRefStyle` replaces the old `in_types_file: bool` parameter with a clear enum
//! that handles the three possible reference contexts.
//!
//! All `render_*` functions return owned `String`s. Type dispatch (`render_res_type`)
//! stays in Rust; structural composition uses boilerplate templates.

use super::templates::{
    EnumVariantRes, PlainEnumDeclRes, RecordFieldRes, RecordTypeDeclRes, SumTypeDeclRes, SumVariantRes, UnitTypeDeclRes,
};
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
// Type rendering (pre-render boundary — stays in Rust, returns String)
// ---------------------------------------------------------------------------

/// Render a ReScript type expression for `AlgebraicTypeUse` into a `String`.
///
/// This is the pre-render boundary: recursive type dispatch must happen in Rust,
/// but the result is a plain string that boilerplate templates embed via `{{...}}`.
pub fn render_res_type(module: &ModuleDef, ty: &AlgebraicTypeUse, style: TypeRefStyle) -> String {
    match ty {
        AlgebraicTypeUse::Unit | AlgebraicTypeUse::Never => "unit".to_string(),
        AlgebraicTypeUse::Identity | AlgebraicTypeUse::ConnectionId | AlgebraicTypeUse::Uuid => "string".to_string(),
        AlgebraicTypeUse::String => "string".to_string(),
        AlgebraicTypeUse::Timestamp => match style {
            TypeRefStyle::InTypesFile | TypeRefStyle::InRecursiveGroup => "timestamp".to_string(),
            TypeRefStyle::External => "StdbTypes.timestamp".to_string(),
        },
        AlgebraicTypeUse::TimeDuration => match style {
            TypeRefStyle::InTypesFile | TypeRefStyle::InRecursiveGroup => "timeDuration".to_string(),
            TypeRefStyle::External => "StdbTypes.timeDuration".to_string(),
        },
        AlgebraicTypeUse::ScheduleAt => match style {
            TypeRefStyle::InTypesFile | TypeRefStyle::InRecursiveGroup => "scheduleAt".to_string(),
            TypeRefStyle::External => "StdbTypes.scheduleAt".to_string(),
        },
        AlgebraicTypeUse::Option(inner) => {
            let inner_str = render_res_type(module, inner, style);
            format!("option<{inner_str}>")
        }
        AlgebraicTypeUse::Result { ok_ty, err_ty } => {
            let ok_str = render_res_type(module, ok_ty, style);
            let err_str = render_res_type(module, err_ty, style);
            format!("result<{ok_str}, {err_str}>")
        }
        AlgebraicTypeUse::Primitive(prim) => match prim {
            PrimitiveType::Bool => "bool".to_string(),
            PrimitiveType::I8
            | PrimitiveType::U8
            | PrimitiveType::I16
            | PrimitiveType::U16
            | PrimitiveType::I32
            | PrimitiveType::U32 => "int".to_string(),
            PrimitiveType::I64
            | PrimitiveType::U64
            | PrimitiveType::I128
            | PrimitiveType::U128
            | PrimitiveType::I256
            | PrimitiveType::U256 => "bigint".to_string(),
            PrimitiveType::F32 | PrimitiveType::F64 => "float".to_string(),
        },
        AlgebraicTypeUse::Array(inner) => {
            let inner_str = render_res_type(module, inner, style);
            format!("array<{inner_str}>")
        }
        AlgebraicTypeUse::Ref(reference) => {
            let pascal_name = type_ref_name(module, *reference);
            let module_name = rescript_module_name(&pascal_name);
            match style {
                TypeRefStyle::InTypesFile => format!("{module_name}.t"),
                TypeRefStyle::InRecursiveGroup => rescript_type_name(pascal_name),
                TypeRefStyle::External => format!("StdbTypes.{module_name}.t"),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Schema AlgebraicType rendering (pre-render boundary — stays in Rust)
// ---------------------------------------------------------------------------

/// Render a ReScript `AlgType.*` expression for `AlgebraicTypeUse`.
///
/// This produces the BSATN-level algebraicType representation consumed by the
/// SDK's `ProductType.makeDeserializer`. Unlike `render_res_type` (which renders
/// ReScript *type expressions*), this renders ReScript *value expressions* that
/// construct `StdbSdk.algebraicType` values at runtime.
///
/// Named types (`Ref`) are rendered as camelCase let-binding names (e.g., `accountType`)
/// because the generated `StdbSchema.res` defines a `let` binding per named type.
pub fn render_schema_alg_type(module: &ModuleDef, ty: &AlgebraicTypeUse) -> String {
    match ty {
        AlgebraicTypeUse::Unit => "AlgType.unit_".to_string(),
        AlgebraicTypeUse::Never => "AlgType.unit_".to_string(),
        AlgebraicTypeUse::Identity => "AlgType.identity".to_string(),
        AlgebraicTypeUse::ConnectionId => "AlgType.connectionId".to_string(),
        AlgebraicTypeUse::Timestamp => "AlgType.timestamp".to_string(),
        AlgebraicTypeUse::TimeDuration => "AlgType.timeDuration".to_string(),
        AlgebraicTypeUse::ScheduleAt => "AlgType.scheduleAt".to_string(),
        AlgebraicTypeUse::Uuid => "AlgType.uuid".to_string(),
        AlgebraicTypeUse::String => "AlgType.string_".to_string(),
        AlgebraicTypeUse::Option(inner) => {
            let inner_str = render_schema_alg_type(module, inner);
            format!("AlgType.option({inner_str})")
        }
        AlgebraicTypeUse::Result { ok_ty, err_ty } => {
            let ok_str = render_schema_alg_type(module, ok_ty);
            let err_str = render_schema_alg_type(module, err_ty);
            format!("AlgType.result({ok_str}, {err_str})")
        }
        AlgebraicTypeUse::Primitive(prim) => match prim {
            PrimitiveType::Bool => "AlgType.bool_".to_string(),
            PrimitiveType::I8 => "AlgType.i8".to_string(),
            PrimitiveType::U8 => "AlgType.u8".to_string(),
            PrimitiveType::I16 => "AlgType.i16".to_string(),
            PrimitiveType::U16 => "AlgType.u16".to_string(),
            PrimitiveType::I32 => "AlgType.i32".to_string(),
            PrimitiveType::U32 => "AlgType.u32".to_string(),
            PrimitiveType::I64 => "AlgType.i64".to_string(),
            PrimitiveType::U64 => "AlgType.u64".to_string(),
            PrimitiveType::I128 => "AlgType.i128".to_string(),
            PrimitiveType::U128 => "AlgType.u128".to_string(),
            PrimitiveType::I256 => "AlgType.i256".to_string(),
            PrimitiveType::U256 => "AlgType.u256".to_string(),
            PrimitiveType::F32 => "AlgType.f32".to_string(),
            PrimitiveType::F64 => "AlgType.f64".to_string(),
        },
        AlgebraicTypeUse::Array(inner) => {
            if matches!(&**inner, AlgebraicTypeUse::Primitive(PrimitiveType::U8)) {
                return "AlgType.byteArray".to_string();
            }
            let inner_str = render_schema_alg_type(module, inner);
            format!("AlgType.array_({inner_str})")
        }
        AlgebraicTypeUse::Ref(r) => {
            // Reference to a named type — rendered as the let-binding name in StdbSchema.res
            let pascal_name = type_ref_name(module, *r);
            schema_type_binding_name(&pascal_name)
        }
    }
}

/// Convert a PascalCase type name to a camelCase let-binding name for schema use.
/// E.g., `AccountType` → `accountType_`, `Receipt` → `receipt_`.
/// We suffix with `_` to avoid clashing with ReScript keywords and other bindings.
pub fn schema_type_binding_name(pascal_name: &str) -> String {
    let camel = pascal_name.to_case(Case::Camel);
    format!("{camel}_")
}

// ---------------------------------------------------------------------------
// Record / Sum / Enum type rendering (boilerplate-based, return String)
// ---------------------------------------------------------------------------

/// Intermediate data for a record field — owns the strings so `RecordFieldRes` can borrow.
struct RecordFieldData {
    raw: String,
    camel: String,
    type_str: String,
}

/// Render `type <name> = { @as("raw") camel: T, ... }` (no `rec`).
pub fn render_record_type(
    module: &ModuleDef,
    name: &str,
    elements: &[(Identifier, AlgebraicTypeUse)],
    style: TypeRefStyle,
) -> String {
    render_record_type_kw(module, "type", name, elements, style)
}

/// Render `<keyword> <name> = { ... }` where keyword is `type`, `type rec`, or `and`.
///
/// Returns the type declaration as a `String` (with trailing newline, no trailing blank line).
pub fn render_record_type_kw(
    module: &ModuleDef,
    keyword: &str,
    name: &str,
    elements: &[(Identifier, AlgebraicTypeUse)],
    style: TypeRefStyle,
) -> String {
    if elements.is_empty() {
        return UnitTypeDeclRes { keyword, name }.to_string();
    }

    // Build owned field data, then borrow for template structs.
    let field_data: Vec<RecordFieldData> = elements
        .iter()
        .map(|(field, ty)| {
            let raw = field.deref().to_string();
            let camel = rescript_field_name(raw.to_case(Case::Camel));
            let type_str = render_res_type(module, ty, style);
            RecordFieldData { raw, camel, type_str }
        })
        .collect();

    let fields: Vec<RecordFieldRes> = field_data
        .iter()
        .map(|f| RecordFieldRes {
            raw_name: &f.raw,
            camel_name: &f.camel,
            type_str: &f.type_str,
        })
        .collect();

    RecordTypeDeclRes { keyword, name, fields }.to_string()
}

/// Intermediate data for a sum variant.
struct SumVariantData {
    constructor: String,
    payload: String,
}

/// Render a sum type with `@tag("tag")` discrimination.
///
/// Returns the type declaration as a `String` (with trailing newline).
pub fn render_sum_type(
    module: &ModuleDef,
    keyword: &str,
    name: &str,
    variants: &[(Identifier, AlgebraicTypeUse)],
    style: TypeRefStyle,
) -> String {
    let variant_data: Vec<SumVariantData> = variants
        .iter()
        .map(|(variant_name, variant_type)| {
            let constructor = rescript_constructor_name(variant_name.deref());
            let payload = if matches!(variant_type, AlgebraicTypeUse::Unit) {
                String::new()
            } else {
                render_res_type(module, variant_type, style)
            };
            SumVariantData { constructor, payload }
        })
        .collect();

    let template_variants: Vec<SumVariantRes> = variant_data
        .iter()
        .map(|v| SumVariantRes {
            constructor: &v.constructor,
            payload: &v.payload,
        })
        .collect();

    SumTypeDeclRes {
        keyword,
        name,
        variants: template_variants,
    }
    .to_string()
}

/// Render a plain enum (all-unit variants, no `@tag` needed — compiles to strings).
///
/// Returns the type declaration as a `String` (with trailing newline).
pub fn render_plain_enum(keyword: &str, name: &str, variants: &[Identifier]) -> String {
    let constructor_names: Vec<String> = variants.iter().map(|v| rescript_constructor_name(v.deref())).collect();

    let template_variants: Vec<EnumVariantRes> = constructor_names
        .iter()
        .map(|c| EnumVariantRes { constructor: c })
        .collect();

    PlainEnumDeclRes {
        keyword,
        name,
        variants: template_variants,
    }
    .to_string()
}
