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

/// Newtype helper functions (make/value/toKey/equal) for single-field products.
/// Emitted inside the module wrapper alongside the type declaration.
#[derive(Boilerplate)]
pub(super) struct NewtypeHelpersRes<'a> {
    /// The camelCase field name (e.g., "id").
    pub field_camel: &'a str,
    /// The ReScript type of the inner field (e.g., "bigint", "string").
    pub inner_type: &'a str,
    /// Pre-rendered `toKey` expression, or None to omit `toKey`.
    /// e.g., `Some("BigInt.toString(v.id)")` for bigint fields.
    pub to_key_expr: Option<&'a str>,
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

/// `SpacetimeDBProvider.res` — React component binding, sdk_module-parameterised.
#[derive(Boilerplate)]
pub(super) struct SpacetimedbProviderRes<'a> {
    pub sdk_module: &'a str,
}

/// `StdbClient.res` — db record aggregating all tables + connection accessors.
#[derive(Boilerplate)]
pub(super) struct StdbClientRes<'a> {
    pub header: AutoGenHeaderRes,
    pub db_fields: Vec<DbFieldRes<'a>>,
    pub sdk_module: &'a str,
}

/// Namespace gateway file (root, tables, reducers, procedures).
/// Renders a flat list of module aliases.
#[derive(Boilerplate)]
pub(super) struct NamespaceGatewayRes<'a> {
    pub header: AutoGenHeaderRes,
    pub aliases: Vec<ModuleAliasRes<'a>>,
}

/// Per-table file: `Stdb__Tables__*.res`.
#[derive(Boilerplate)]
pub(super) struct TableFileRes<'a> {
    pub header: AutoGenHeaderRes,
    /// Pre-rendered row record type block.
    pub row_type: &'a str,
    /// Pre-rendered PK index section, or empty string if no PK.
    pub pk_section: &'a str,
    pub table_name: &'a str,
    /// Pre-rendered typed event union + subscribe (always present).
    pub event_section: &'a str,
    /// Pre-rendered observer functor section, or empty string when async_style = Promise.
    pub observer_section: &'a str,
    /// Pre-rendered React hooks section, or empty string when async_style = Observer.
    pub react_hooks: &'a str,
    pub sdk_module: &'a str,
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
    /// Pre-rendered React hooks section, or empty string when async_style = Observer.
    pub react_hooks: &'a str,
    /// Pre-rendered Make functor section, or empty string when async_style = Promise.
    pub make_functor: &'a str,
    pub sdk_module: &'a str,
}

/// Per-reducer file (no args): `Stdb*Reducer.res`.
#[derive(Boilerplate)]
pub(super) struct ReducerNoArgsFileRes<'a> {
    pub header: AutoGenHeaderRes,
    pub accessor: &'a str,
    /// Pre-rendered React hooks section, or empty string when async_style = Observer.
    pub react_hooks: &'a str,
    /// Pre-rendered Make functor section, or empty string when async_style = Promise.
    pub make_functor: &'a str,
    pub sdk_module: &'a str,
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
pub(super) struct TypesPreambleRes<'a> {
    pub sdk_module: &'a str,
}

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
    pub sdk_module: &'a str,
}

// ===========================================================================
// Display Layer 0: Atomic pieces for StdbDisplay.res
// ===========================================================================

/// A single match arm in an enum toString function.
/// Renders: `  | Constructor => "Constructor"`
#[derive(Boilerplate)]
pub(super) struct DisplayEnumArmRes<'a> {
    pub constructor: &'a str,
}

// ===========================================================================
// Display Layer 1: Structural sections for StdbDisplay.res
// ===========================================================================

/// A single newtype unwrapper function.
/// Renders: `let fnName = (v: ModuleName.t) => v.fieldName`
#[derive(Boilerplate)]
pub(super) struct DisplayUnwrapperRes<'a> {
    pub fn_name: &'a str,
    pub module_name: &'a str,
    pub field_name: &'a str,
}

/// A single enum toString function with switch arms.
#[derive(Boilerplate)]
pub(super) struct DisplayEnumToStringRes<'a> {
    pub fn_name: &'a str,
    pub module_name: &'a str,
    pub arms: Vec<DisplayEnumArmRes<'a>>,
}

// ===========================================================================
// Display Layer 2: File Composition
// ===========================================================================

/// `StdbDisplay.res` — mechanical unwrappers and toString functions.
#[derive(Boilerplate)]
pub(super) struct StdbDisplayRes<'a> {
    pub header: AutoGenHeaderRes,
    /// Pre-rendered newtype unwrapper lines.
    pub unwrappers: &'a str,
    /// Pre-rendered enum toString functions.
    pub enum_to_strings: &'a str,
    pub sdk_module: &'a str,
}

// ===========================================================================
// Schema Layer 0: Atomic pieces for StdbSchema.res
// ===========================================================================

/// A single product element in a schema type builder.
/// Renders: `{name: Some("fieldName"), algebraicType: algTypeExpr},`
#[derive(Boilerplate)]
pub(super) struct SchemaProductElementRes<'a> {
    pub field_name: &'a str,
    pub alg_type_expr: &'a str,
}

/// A single sum variant in a schema type builder.
/// Renders: `{name: Some("VariantName"), algebraicType: algTypeExpr},`
#[derive(Boilerplate)]
pub(super) struct SchemaVariantElementRes<'a> {
    pub variant_name: &'a str,
    pub alg_type_expr: &'a str,
}

/// A single column entry in a table's columns Dict.
/// Renders: `("colName", {columnMetadata: {...}, typeBuilder: {algebraicType: expr}}),`
#[derive(Boilerplate)]
pub(super) struct SchemaColumnEntryRes<'a> {
    pub col_name: &'a str,
    pub is_primary_key: bool,
    pub alg_type_expr: &'a str,
}

/// A single index entry in a table's indexes array.
/// Renders: `{name: "idxName", algorithm: "btree", columns: ["col1", "col2"]},`
#[derive(Boilerplate)]
pub(super) struct SchemaIndexEntryRes<'a> {
    pub index_name: &'a str,
    /// Pre-rendered columns array content, e.g. `"col1", "col2"`.
    pub columns_str: &'a str,
}

/// A single constraint entry in a table's constraints array.
/// Renders: `{constraintName: "name", constraint: "unique", columns: ["col1"]},`
#[derive(Boilerplate)]
pub(super) struct SchemaConstraintEntryRes<'a> {
    pub constraint_name: &'a str,
    /// Pre-rendered columns array content, e.g. `"col1", "col2"`.
    pub columns_str: &'a str,
}

// ===========================================================================
// Schema Layer 1: Structural sections for StdbSchema.res
// ===========================================================================

/// A named type binding — product type.
/// Renders: `let typeName_ = Compound(Product({value: {elements: [...]}}))`
#[derive(Boilerplate)]
pub(super) struct SchemaProductBindingRes<'a> {
    pub binding_name: &'a str,
    pub elements: Vec<SchemaProductElementRes<'a>>,
}

/// A named type binding — sum type (tagged union).
/// Renders: `let typeName_ = Compound(Sum({value: {variants: [...]}}))`
#[derive(Boilerplate)]
pub(super) struct SchemaSumBindingRes<'a> {
    pub binding_name: &'a str,
    pub variants: Vec<SchemaVariantElementRes<'a>>,
}

/// A single table definition entry in the remoteModule.
/// Renders the full `("accessorName", {...tableDef}),` tuple.
#[derive(Boilerplate)]
pub(super) struct SchemaTableEntryRes<'a> {
    pub accessor_name: &'a str,
    pub source_name: &'a str,
    pub row_elements: Vec<SchemaProductElementRes<'a>>,
    pub columns: Vec<SchemaColumnEntryRes<'a>>,
    pub indexes: Vec<SchemaIndexEntryRes<'a>>,
    pub constraints: Vec<SchemaConstraintEntryRes<'a>>,
    pub is_event: bool,
}

/// A single reducer definition entry.
/// Renders: `{name: "reducer_name", accessorName: "reducerName", paramsType: {elements: [...]}},`
#[derive(Boilerplate)]
pub(super) struct SchemaReducerEntryRes<'a> {
    pub reducer_name: &'a str,
    pub accessor_name: &'a str,
    pub param_elements: Vec<SchemaProductElementRes<'a>>,
}

/// A single procedure definition entry.
/// Renders: `{name: "proc_name", accessorName: "procName"},`
#[derive(Boilerplate)]
pub(super) struct SchemaProcedureEntryRes<'a> {
    pub procedure_name: &'a str,
    pub accessor_name: &'a str,
}

// ===========================================================================
// Schema Layer 2: File Composition
// ===========================================================================

/// `StdbSchema.res` — pure ReScript runtime schema (replaces StdbSchema.mjs).
///
/// Constructs `remoteModule` directly using `sdk_module` types and direct algebraicType constructors.
/// No SDK builder functions — just record literals.
#[derive(Boilerplate)]
pub(super) struct StdbSchemaRes<'a> {
    pub header: AutoGenHeaderRes,
    pub cli_version: &'a str,
    /// Pre-rendered type bindings (let typeName_ = Compound(Product/Sum(...)))
    pub type_bindings: &'a str,
    pub table_entries: Vec<SchemaTableEntryRes<'a>>,
    pub reducer_entries: Vec<SchemaReducerEntryRes<'a>>,
    pub procedure_entries: Vec<SchemaProcedureEntryRes<'a>>,
    /// Comma-separated list of all table accessor names for allTableNames.
    pub all_table_names: Vec<&'a str>,
    pub sdk_module: &'a str,
}

// ===========================================================================
// Async/Observer Layer: Stdb__Async.res and functor sections
// ===========================================================================

/// `Stdb__Async.res` — EFFECT_RUNTIME + OBSERVER module type contracts.
/// Entirely static — no schema data.
#[derive(Boilerplate)]
pub(super) struct StdbAsyncRes;

/// Typed event union section for table files.
/// Emitted unconditionally (regardless of async_style).
/// Contains: type event, let subscribe
#[derive(Boilerplate)]
pub(super) struct TableEventSectionRes<'a> {
    pub sdk_module: &'a str,
}

/// Observer functor section for table files.
/// Emitted when async_style ∈ {Observer, All}.
/// Contains: module MakeStream with observe + observeWithCtx
#[derive(Boilerplate)]
pub(super) struct TableObserverSectionRes<'a> {
    pub sdk_module: &'a str,
}

/// Make functor section for reducer files (with or without args).
/// Emitted when async_style ∈ {Observer, All}.
/// Contains: module Make = (E: Stdb__Async.EFFECT_RUNTIME) => { let call ... }
#[derive(Boilerplate)]
pub(super) struct ReducerMakeFunctorRes<'a> {
    pub accessor: &'a str,
    pub has_args: bool,
    pub sdk_module: &'a str,
}
