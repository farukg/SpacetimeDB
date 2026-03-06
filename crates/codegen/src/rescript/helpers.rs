//! Shared ReScript codegen helpers: type rendering, name munging, record/sum emission.
//!
//! `TypeRefStyle` replaces the old `in_types_file: bool` parameter with a clear enum
//! that handles the three possible reference contexts.
//!
//! All `render_*` functions return owned `String`s. Type dispatch (`render_res_type`)
//! stays in Rust; structural composition uses boilerplate templates.

use crate::util::type_ref_name;

use convert_case::{Case, Casing};
use spacetimedb_lib::sats::layout::PrimitiveType;
use spacetimedb_schema::def::ModuleDef;
use spacetimedb_schema::identifier::Identifier;
use spacetimedb_schema::type_for_generate::AlgebraicTypeUse;
use std::ops::Deref;

use sigma_rescript_codegen::names as shared_names;
use sigma_rescript_codegen::types as shared_types;

/// How to render a reference to another type (`AlgebraicTypeUse::Ref`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeRefStyle {
    /// Inside `StdbTypes.res`, sibling module reference: `AccountId.t`
    InTypesFile,
    /// Inside a `module Recursive_N = { ... }` block: bare name (e.g., `expr`)
    InRecursiveGroup,
    /// From per-entity files that `open {root_module}`: bare `Sdk.identity`, `Types.Foo.t`
    /// The gateway open brings all submodule aliases into scope.
    ViaGateway,
}

// ---------------------------------------------------------------------------
// Name munging
// ---------------------------------------------------------------------------

/// Convert a PascalCase type name to a ReScript module name (PascalCase, escaping reserved words).
pub fn rescript_module_name(name: &str) -> String {
    shared_names::rescript_module_name(name)
}

pub fn rescript_type_name(type_name_pascal: String) -> String {
    shared_names::rescript_type_name(&type_name_pascal)
}

pub fn rescript_field_name(name: String) -> String {
    shared_names::rescript_field_name(&name)
}

pub fn rescript_constructor_name(name: &str) -> String {
    shared_names::rescript_constructor_name(name)
}

pub fn table_module_name(root_module: &str, table_name: &Identifier) -> String {
    format!(
        "{root_module}__Tables__{name}",
        name = table_name.deref().to_case(Case::Pascal)
    )
}

pub fn reducer_module_name(root_module: &str, reducer_name: &spacetimedb_schema::reducer_name::ReducerName) -> String {
    format!(
        "{root_module}__Reducers__{name}",
        name = reducer_name.deref().to_case(Case::Pascal)
    )
}

pub fn procedure_module_name(root_module: &str, procedure_name: &Identifier) -> String {
    format!(
        "{root_module}__Procedures__{name}",
        name = procedure_name.deref().to_case(Case::Pascal)
    )
}

// ---------------------------------------------------------------------------
// Type rendering (pre-render boundary — stays in Rust, returns String)
// ---------------------------------------------------------------------------

/// Render a ReScript type expression for `AlgebraicTypeUse` into a `String`.
///
/// This is the pre-render boundary: recursive type dispatch must happen in Rust,
/// but the result is a plain string that boilerplate templates embed via `{{...}}`.
pub fn render_res_type(module: &ModuleDef, ty: &AlgebraicTypeUse, style: TypeRefStyle, root_module: &str) -> String {
    match ty {
        AlgebraicTypeUse::Unit | AlgebraicTypeUse::Never => "unit".to_string(),
        AlgebraicTypeUse::Identity => match style {
            TypeRefStyle::InTypesFile | TypeRefStyle::InRecursiveGroup => "identity".to_string(),
            TypeRefStyle::ViaGateway => "Sdk.identity".to_string(),
        },
        AlgebraicTypeUse::ConnectionId => match style {
            TypeRefStyle::InTypesFile | TypeRefStyle::InRecursiveGroup => "connectionId".to_string(),
            TypeRefStyle::ViaGateway => "Sdk.connectionId".to_string(),
        },
        AlgebraicTypeUse::Uuid => match style {
            TypeRefStyle::InTypesFile | TypeRefStyle::InRecursiveGroup => "uuid".to_string(),
            TypeRefStyle::ViaGateway => "Sdk.uuid".to_string(),
        },
        AlgebraicTypeUse::String => "string".to_string(),
        AlgebraicTypeUse::Timestamp => match style {
            TypeRefStyle::InTypesFile | TypeRefStyle::InRecursiveGroup => "timestamp".to_string(),
            TypeRefStyle::ViaGateway => "Sdk.timestamp".to_string(),
        },
        AlgebraicTypeUse::TimeDuration => match style {
            TypeRefStyle::InTypesFile | TypeRefStyle::InRecursiveGroup => "timeDuration".to_string(),
            TypeRefStyle::ViaGateway => "Sdk.timeDuration".to_string(),
        },
        AlgebraicTypeUse::ScheduleAt => match style {
            TypeRefStyle::InTypesFile | TypeRefStyle::InRecursiveGroup => "scheduleAt".to_string(),
            TypeRefStyle::ViaGateway => "Sdk.scheduleAt".to_string(),
        },
        AlgebraicTypeUse::Option(inner) => {
            let inner_str = render_res_type(module, inner, style, root_module);
            format!("option<{inner_str}>")
        }
        AlgebraicTypeUse::Result { ok_ty, err_ty } => {
            let ok_str = render_res_type(module, ok_ty, style, root_module);
            let err_str = render_res_type(module, err_ty, style, root_module);
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
            let inner_str = render_res_type(module, inner, style, root_module);
            format!("array<{inner_str}>")
        }
        AlgebraicTypeUse::Ref(reference) => {
            let pascal_name = type_ref_name(module, *reference);
            let module_name = rescript_module_name(&pascal_name);
            match style {
                TypeRefStyle::InTypesFile => format!("{module_name}.t"),
                TypeRefStyle::InRecursiveGroup => rescript_type_name(pascal_name),
                TypeRefStyle::ViaGateway => format!("Types.{module_name}.t"),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Schema AlgebraicType rendering (pre-render boundary — stays in Rust)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Schema algebra helpers — compose ReScript algebraicType value expressions
// ---------------------------------------------------------------------------

/// Wrap elements into a `Compound(Product({value: {elements: [...]}}))` expression.
fn alg_product(elements: &[String]) -> String {
    format!("Compound(Product({{value: {{elements: [{}]}}}}))", elements.join(", "))
}

/// Create a product element `{name: Some("n"), algebraicType: ty}`.
fn alg_element(name: &str, ty: &str) -> String {
    format!("{{name: Some(\"{name}\"), algebraicType: {ty}}}")
}

/// Wrap variants into a `Compound(Sum({value: {variants: [...]}}))` expression.
fn alg_sum(variants: &[String]) -> String {
    format!("Compound(Sum({{value: {{variants: [{}]}}}}))", variants.join(", "))
}

/// Create a sum variant `{name: Some("n"), algebraicType: ty}`.
fn alg_variant(name: &str, ty: &str) -> String {
    format!("{{name: Some(\"{name}\"), algebraicType: {ty}}}")
}

/// Render a ReScript algebraicType value expression for `AlgebraicTypeUse`.
///
/// This produces the BSATN-level algebraicType representation consumed by the
/// SDK's `ProductType.makeDeserializer`. Uses direct constructors from the
/// two-tier `@unboxed` + `@tag("tag")` design:
/// - Primitives: bare constructors like `U8`, `Bool`, `String`
/// - Compounds: `Compound(Product({value: ...}))`, `Compound(Sum({value: ...}))`, etc.
///
/// Named types (`Ref`) are rendered as camelCase let-binding names (e.g., `accountType`)
/// because the generated `StdbSchema.res` defines a `let` binding per named type.
pub fn render_schema_alg_type(module: &ModuleDef, ty: &AlgebraicTypeUse) -> String {
    match ty {
        AlgebraicTypeUse::Unit => alg_product(&[]),
        AlgebraicTypeUse::Never => alg_product(&[]),
        AlgebraicTypeUse::Identity => alg_product(&[alg_element("__identity__", "U256")]),
        AlgebraicTypeUse::ConnectionId => alg_product(&[alg_element("__connection_id__", "U128")]),
        AlgebraicTypeUse::Timestamp => alg_product(&[alg_element("__timestamp_micros_since_unix_epoch__", "I64")]),
        AlgebraicTypeUse::TimeDuration => alg_product(&[alg_element("__time_duration_micros__", "I64")]),
        AlgebraicTypeUse::ScheduleAt => {
            let time_duration = alg_product(&[alg_element("__time_duration_micros__", "I64")]);
            let timestamp = alg_product(&[alg_element("__timestamp_micros_since_unix_epoch__", "I64")]);
            alg_sum(&[alg_variant("Interval", &time_duration), alg_variant("Time", &timestamp)])
        }
        AlgebraicTypeUse::Uuid => alg_product(&[alg_element("__uuid__", "U128")]),
        AlgebraicTypeUse::String => "String".to_string(),
        AlgebraicTypeUse::Option(inner) => {
            let inner_str = render_schema_alg_type(module, inner);
            let none = alg_product(&[]);
            alg_sum(&[alg_variant("some", &inner_str), alg_variant("none", &none)])
        }
        AlgebraicTypeUse::Result { ok_ty, err_ty } => {
            let ok_str = render_schema_alg_type(module, ok_ty);
            let err_str = render_schema_alg_type(module, err_ty);
            alg_sum(&[alg_variant("ok", &ok_str), alg_variant("err", &err_str)])
        }
        AlgebraicTypeUse::Primitive(prim) => match prim {
            PrimitiveType::Bool => "Bool".to_string(),
            PrimitiveType::I8 => "I8".to_string(),
            PrimitiveType::U8 => "U8".to_string(),
            PrimitiveType::I16 => "I16".to_string(),
            PrimitiveType::U16 => "U16".to_string(),
            PrimitiveType::I32 => "I32".to_string(),
            PrimitiveType::U32 => "U32".to_string(),
            PrimitiveType::I64 => "I64".to_string(),
            PrimitiveType::U64 => "U64".to_string(),
            PrimitiveType::I128 => "I128".to_string(),
            PrimitiveType::U128 => "U128".to_string(),
            PrimitiveType::I256 => "I256".to_string(),
            PrimitiveType::U256 => "U256".to_string(),
            PrimitiveType::F32 => "F32".to_string(),
            PrimitiveType::F64 => "F64".to_string(),
        },
        AlgebraicTypeUse::Array(inner) => {
            if matches!(&**inner, AlgebraicTypeUse::Primitive(PrimitiveType::U8)) {
                return format!("Compound(Array({{value: U8}}))");
            }
            let inner_str = render_schema_alg_type(module, inner);
            format!("Compound(Array({{value: {inner_str}}}))")
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

/// Pre-render `module Alias = {root}__Alias` lines for sibling module imports.
///
/// Replaces `open {root_module}` in generated files, which creates circular dependencies
/// because the root gateway re-exports all siblings (e.g. `Stdb.res` has
/// `module Types = Stdb__Types` etc., so `open Stdb` → cycle).
///
/// Each generated file instead declares only the specific sibling aliases it needs.
pub fn sibling_opens(root_module: &str, siblings: &[&str]) -> String {
    shared_names::sibling_opens(root_module, siblings)
}

// ---------------------------------------------------------------------------
// Newtype helpers (A2)
// ---------------------------------------------------------------------------

/// Compute the `toKey` expression for a single-field newtype.
///
/// Returns `Some("BigInt.toString(v.field)")` for bigint fields, etc.
/// Returns `None` for types where a `toKey` function doesn't make sense
/// (option, result, array, scheduleAt, Ref to another product type, etc.).
pub fn render_to_key_expr(ty: &AlgebraicTypeUse, field_camel: &str) -> Option<String> {
    match ty {
        AlgebraicTypeUse::Primitive(prim) => match prim {
            PrimitiveType::I64
            | PrimitiveType::U64
            | PrimitiveType::I128
            | PrimitiveType::U128
            | PrimitiveType::I256
            | PrimitiveType::U256 => Some(format!("BigInt.toString(v.{field_camel})")),
            PrimitiveType::I8
            | PrimitiveType::U8
            | PrimitiveType::I16
            | PrimitiveType::U16
            | PrimitiveType::I32
            | PrimitiveType::U32 => Some(format!("Int.toString(v.{field_camel})")),
            PrimitiveType::F32 | PrimitiveType::F64 => Some(format!("Float.toString(v.{field_camel})")),
            PrimitiveType::Bool => Some(format!("string_of_bool(v.{field_camel})")),
        },
        AlgebraicTypeUse::String | AlgebraicTypeUse::Identity | AlgebraicTypeUse::Uuid => {
            Some(format!("v.{field_camel}"))
        }
        AlgebraicTypeUse::ConnectionId
        | AlgebraicTypeUse::Timestamp
        | AlgebraicTypeUse::TimeDuration
        | AlgebraicTypeUse::ScheduleAt
        | AlgebraicTypeUse::Option(_)
        | AlgebraicTypeUse::Result { .. }
        | AlgebraicTypeUse::Array(_)
        | AlgebraicTypeUse::Ref(_)
        | AlgebraicTypeUse::Unit
        | AlgebraicTypeUse::Never => None,
    }
}

// ---------------------------------------------------------------------------
// Equality expression for newtype helpers (W102-safe)
// ---------------------------------------------------------------------------

/// Compute a W102-safe equality expression for a single-field newtype.
///
/// Primitives and string-like types use `===` (physical equality, no W102).
/// Refs to other newtypes use `ModuleName.equal(a.field, b.field)` (delegates
/// to that type's own `equal` function, also W102-safe).
/// Returns `None` for types where a meaningful `equal` doesn't apply
/// (option, result, array, etc.).
pub fn render_equal_expr(module: &ModuleDef, ty: &AlgebraicTypeUse, field_camel: &str) -> Option<String> {
    match ty {
        // JS primitives — `===` is safe and avoids W102
        AlgebraicTypeUse::Primitive(_)
        | AlgebraicTypeUse::String
        | AlgebraicTypeUse::Identity
        | AlgebraicTypeUse::ConnectionId
        | AlgebraicTypeUse::Uuid => Some(format!("a.{field_camel} === b.{field_camel}")),
        // Ref to another named type — delegate to its `equal` function
        AlgebraicTypeUse::Ref(reference) => {
            let pascal_name = type_ref_name(module, *reference);
            let mod_name = rescript_module_name(&pascal_name);
            Some(format!("{mod_name}.equal(a.{field_camel}, b.{field_camel})"))
        }
        // Complex/unsupported types — no meaningful equality
        AlgebraicTypeUse::Timestamp
        | AlgebraicTypeUse::TimeDuration
        | AlgebraicTypeUse::ScheduleAt
        | AlgebraicTypeUse::Option(_)
        | AlgebraicTypeUse::Result { .. }
        | AlgebraicTypeUse::Array(_)
        | AlgebraicTypeUse::Unit
        | AlgebraicTypeUse::Never => None,
    }
}

// ---------------------------------------------------------------------------
// Record / Sum / Enum type rendering (boilerplate-based, return String)
// ---------------------------------------------------------------------------

/// Render `type <name> = { @as("raw") camel: T, ... }` (no `rec`).
pub fn render_record_type(
    module: &ModuleDef,
    name: &str,
    elements: &[(Identifier, AlgebraicTypeUse)],
    style: TypeRefStyle,
    root_module: &str,
) -> String {
    render_record_type_kw(module, "type", name, elements, style, root_module)
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
    root_module: &str,
) -> String {
    let rendered_fields: Vec<(String, String)> = elements
        .iter()
        .map(|(field, ty)| {
            let raw = field.deref().to_string();
            let type_str = render_res_type(module, ty, style, root_module);
            (raw, type_str)
        })
        .collect();

    let field_refs: Vec<(&str, &str)> = rendered_fields
        .iter()
        .map(|(raw, type_str)| (raw.as_str(), type_str.as_str()))
        .collect();

    shared_types::render_record_type(keyword, name, &field_refs)
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
    root_module: &str,
) -> String {
    let rendered_variants: Vec<(String, String)> = variants
        .iter()
        .map(|(variant_name, variant_type)| {
            let constructor = rescript_constructor_name(variant_name.deref());
            let payload = if matches!(variant_type, AlgebraicTypeUse::Unit) {
                String::new()
            } else {
                render_res_type(module, variant_type, style, root_module)
            };
            (constructor, payload)
        })
        .collect();

    let variant_refs: Vec<(&str, &str)> = rendered_variants
        .iter()
        .map(|(constructor, payload)| (constructor.as_str(), payload.as_str()))
        .collect();

    shared_types::render_sum_type(keyword, name, &variant_refs)
}

/// Render a plain enum (all-unit variants, no `@tag` needed — compiles to strings).
///
/// Returns the type declaration as a `String` (with trailing newline).
pub fn render_plain_enum(keyword: &str, name: &str, variants: &[Identifier]) -> String {
    let constructor_names: Vec<String> = variants.iter().map(|v| rescript_constructor_name(v.deref())).collect();
    let variant_refs: Vec<&str> = constructor_names.iter().map(|v| v.as_str()).collect();
    shared_types::render_plain_enum(keyword, name, &variant_refs)
}
