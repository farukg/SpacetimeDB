//! Boilerplate template structs for ReScript codegen.
//!
//! Organized in 3 layers:
//! - **Layer 0 — Atomic Primitives**: Smallest reusable pieces (record field, variant, module alias).
//! - **Layer 1 — Structural Sections**: Meaningful blocks composed from Layer 0 (type decls, PK index, hooks).
//! - **Layer 2 — File Composition**: One struct per output file, composing Layer 0+1 via nested `Display`.
//!
//! Every `#[derive(Boilerplate)]` struct implements `Display`. Parent templates embed children
//! via `{{self.child}}` (direct) or `%% for item in &self.vec { {{item}} %% }` (iteration).
//!
//! Pre-rendered `&str` fields are used where recursive Rust type dispatch (`render_res_type`)
//! must intervene before templating.
//!
//! **All output is `.res` (ReScript). No `.mjs` generation.**

use boilerplate::Boilerplate;

// ===========================================================================
// Layer 0: Atomic Primitives
// ===========================================================================

/// Auto-generated file header comment (2 lines).
#[derive(Boilerplate)]
pub(super) struct AutoGenHeaderRes;

/// A single record field with `@as` annotation.
/// Renders: `  @as("raw_name") camelName: typeStr,`
#[derive(Boilerplate)]
pub(super) struct RecordFieldRes<'a> {
    pub raw_name: &'a str,
    pub camel_name: &'a str,
    /// Pre-rendered type expression (from `render_res_type`).
    pub type_str: &'a str,
}

/// A module alias line.
/// Renders: `module Alias = TargetModule`
#[derive(Boilerplate)]
pub(super) struct ModuleAliasRes<'a> {
    pub alias: &'a str,
    pub target: &'a str,
}

/// A `type db` field in StdbClient.
/// Renders: `  @as("accessor") camel: table_module.handle,`
#[derive(Boilerplate)]
pub(super) struct DbFieldRes<'a> {
    pub accessor: &'a str,
    pub camel: &'a str,
    pub table_module: &'a str,
}

/// A sum type variant — unit or with payload.
/// Renders: `| Constructor` or `| Constructor(payload_type)`
#[derive(Boilerplate)]
pub(super) struct SumVariantRes<'a> {
    pub constructor: &'a str,
    /// Pre-rendered payload type, or empty string for unit variants.
    pub payload: &'a str,
}

/// A plain enum variant line (always unit).
/// Renders: `| Constructor`
#[derive(Boilerplate)]
pub(super) struct EnumVariantRes<'a> {
    pub constructor: &'a str,
}

// ===========================================================================
// Layer 1: Structural Sections
// ===========================================================================

/// A record type declaration: `keyword name = { fields... }`
#[derive(Boilerplate)]
pub(super) struct RecordTypeDeclRes<'a> {
    pub keyword: &'a str,
    pub name: &'a str,
    pub fields: Vec<RecordFieldRes<'a>>,
}

/// An empty record type: `keyword name = unit`
#[derive(Boilerplate)]
pub(super) struct UnitTypeDeclRes<'a> {
    pub keyword: &'a str,
    pub name: &'a str,
}

/// A sum type with `@tag("tag")` discrimination.
#[derive(Boilerplate)]
pub(super) struct SumTypeDeclRes<'a> {
    pub keyword: &'a str,
    pub name: &'a str,
    pub variants: Vec<SumVariantRes<'a>>,
}

/// A plain enum (all-unit variants, no `@tag`).
#[derive(Boilerplate)]
pub(super) struct PlainEnumDeclRes<'a> {
    pub keyword: &'a str,
    pub name: &'a str,
    pub variants: Vec<EnumVariantRes<'a>>,
}

/// A module wrapper: `module Name = { content }`
#[derive(Boilerplate)]
pub(super) struct ModuleWrapperRes<'a> {
    pub name: &'a str,
    /// Pre-rendered inner content (type declarations etc).
    pub content: &'a str,
}

/// A module type alias: `module Name = { type t = Group.alias }`
#[derive(Boilerplate)]
pub(super) struct ModuleTypeAliasRes<'a> {
    pub name: &'a str,
    pub group_module: &'a str,
    pub type_alias: &'a str,
}

/// PK index section for a table file.
#[derive(Boilerplate)]
pub(super) struct PkIndexSectionRes<'a> {
    pub field_camel: &'a str,
    pub field_raw: &'a str,
    pub find_param_type: &'a str,
}

/// React table hook section (appears in every table file).
#[derive(Boilerplate)]
pub(super) struct TableReactHookSectionRes<'a> {
    pub accessor: &'a str,
}

/// React reducer hook section (appears in every reducer file).
#[derive(Boilerplate)]
pub(super) struct ReducerReactHookSectionRes<'a> {
    pub params_type: &'a str,
    pub camel_accessor: &'a str,
}

/// A single server reducer async wrapper function.
#[derive(Boilerplate)]
pub(super) struct ServerReducerWrapperRes<'a> {
    pub name_camel: &'a str,
    pub module: &'a str,
    pub has_args: bool,
}

/// Server reducer type record field.
#[derive(Boilerplate)]
pub(super) struct ServerReducerTypeFieldRes<'a> {
    pub name_camel: &'a str,
    pub module: &'a str,
    pub has_args: bool,
}

/// Server reducer value record field.
#[derive(Boilerplate)]
pub(super) struct ServerReducerValueFieldRes<'a> {
    pub name_camel: &'a str,
}

// ===========================================================================
// Layer 2: File Composition
// ===========================================================================

/// `StdbReact.res` — entirely static, no schema data.
#[derive(Boilerplate)]
pub(super) struct StdbReactRes;

/// `SpacetimeDBProvider.res` — entirely static React component binding.
#[derive(Boilerplate)]
pub(super) struct SpacetimedbProviderRes;

/// `StdbClient.res` — db record aggregating all tables + connection accessors.
#[derive(Boilerplate)]
pub(super) struct StdbClientRes<'a> {
    pub header: AutoGenHeaderRes,
    pub db_fields: Vec<DbFieldRes<'a>>,
}

/// `index.res` — module re-exports.
#[derive(Boilerplate)]
pub(super) struct IndexRes<'a> {
    pub header: AutoGenHeaderRes,
    pub table_aliases: Vec<ModuleAliasRes<'a>>,
    pub reducer_aliases: Vec<ModuleAliasRes<'a>>,
    pub procedure_aliases: Vec<ModuleAliasRes<'a>>,
}

/// Per-table file: `Stdb*Table.res`.
#[derive(Boilerplate)]
pub(super) struct TableFileRes<'a> {
    pub header: AutoGenHeaderRes,
    /// Pre-rendered row record type block.
    pub row_type: &'a str,
    pub has_deleted_at: bool,
    /// Pre-rendered PK index section, or empty string if no PK.
    pub pk_section: &'a str,
    pub table_name: &'a str,
    pub react_hooks: TableReactHookSectionRes<'a>,
}

/// Per-reducer file (with args): `Stdb*Reducer.res`.
#[derive(Boilerplate)]
pub(super) struct ReducerWithArgsFileRes<'a> {
    pub header: AutoGenHeaderRes,
    /// Pre-rendered args record type block.
    pub args_record: &'a str,
    pub accessor: &'a str,
    /// Pre-rendered labeled params for `let call`.
    pub call_params: &'a str,
    /// Pre-rendered record construction fields for `let call` body.
    pub call_body_fields: &'a str,
    pub react_hooks: ReducerReactHookSectionRes<'a>,
}

/// Per-reducer file (no args): `Stdb*Reducer.res`.
#[derive(Boilerplate)]
pub(super) struct ReducerNoArgsFileRes<'a> {
    pub header: AutoGenHeaderRes,
    pub accessor: &'a str,
    pub react_hooks: ReducerReactHookSectionRes<'a>,
}

/// Per-procedure file: `Stdb*Procedure.res`.
#[derive(Boilerplate)]
pub(super) struct ProcedureFileRes<'a> {
    pub header: AutoGenHeaderRes,
    /// Pre-rendered params record type block.
    pub params_record: &'a str,
    /// Pre-rendered result type expression.
    pub result_type: &'a str,
    pub procedure_name: &'a str,
}

/// `StdbTypes.res` preamble — opaque SDK types (emitted before per-type modules).
#[derive(Boilerplate)]
pub(super) struct TypesPreambleRes;

/// `StdbTypes.res` postamble — connectionBuilder type (emitted after per-type modules).
#[derive(Boilerplate)]
pub(super) struct TypesPostambleRes;

/// `StdbServerReducers.res` — server-side reducer wrappers with connection management.
#[derive(Boilerplate)]
pub(super) struct StdbServerReducersRes<'a> {
    pub header: AutoGenHeaderRes,
    pub reducer_wrappers: Vec<ServerReducerWrapperRes<'a>>,
    pub reducer_type_fields: Vec<ServerReducerTypeFieldRes<'a>>,
    pub reducer_value_fields: Vec<ServerReducerValueFieldRes<'a>>,
    pub has_reducers: bool,
}
