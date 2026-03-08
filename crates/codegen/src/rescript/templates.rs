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

pub(super) use sigma_rescript_codegen::templates::{
    AutoGenHeaderRes, ModuleAliasRes, ModuleTypeAliasRes, ModuleWrapperRes, NewtypeHelpersRes,
};

// ===========================================================================
// Layer 0: Atomic Primitives
// ===========================================================================

/// A `type db` field in StdbClient.
/// Renders: `  @as("accessor") camel: table_module.handle,`
#[derive(Boilerplate)]
pub(super) struct DbFieldRes<'a> {
    pub accessor: &'a str,
    pub camel: &'a str,
    pub table_module: &'a str,
}

/// A `type reducers` field in StdbClient.
/// Renders: `  @as("accessor") camel: args_type => promise<unit>,`
/// `accessor` = raw camelCase JS runtime key, `camel` = escaped ReScript field name.
#[derive(Boilerplate)]
pub(super) struct ReducerFieldRes<'a> {
    pub accessor: &'a str,
    pub camel: &'a str,
    pub has_args: bool,
    /// Module-qualified args type, e.g. `Stdb__Reducers__Foo.args`.
    /// Empty when `has_args` is false.
    pub args_type: &'a str,
}

/// A `type procedures` field in StdbClient.
/// Renders: `  @as("accessor") camel: params_type => promise<response_type>,`
/// `accessor` = raw camelCase JS runtime key, `camel` = escaped ReScript field name.
#[derive(Boilerplate)]
pub(super) struct ProcedureFieldRes<'a> {
    pub accessor: &'a str,
    pub camel: &'a str,
    pub has_args: bool,
    /// Module-qualified params type, e.g. `Stdb__Procedures__Foo.params`.
    /// Empty when `has_args` is false.
    pub params_type: &'a str,
    /// Module-qualified response type, e.g. `Stdb__Procedures__Foo.response`.
    pub response_type: &'a str,
}

// ===========================================================================
// Layer 1: Structural Sections
// ===========================================================================

/// PK index section for a table file.
#[derive(Boilerplate)]
pub(super) struct PkIndexSectionRes<'a> {
    pub field_camel: &'a str,
    pub field_raw: &'a str,
    pub find_param_type: &'a str,
}

/// React table hook section (appears in every table file).
/// `react_module` is hardcoded as `React` in the template (via gateway open).
#[derive(Boilerplate)]
pub(super) struct TableReactHookSectionRes<'a> {
    pub accessor: &'a str,
    pub has_pk: bool,
    pub pk_type: &'a str,
    pub pk_field_camel: &'a str,
    pub pk_is_identity: bool,
    pub has_display: bool,
    /// Relative path to the schema module's  file (e.g. ).
    pub schema_module_path: &'a str,
}

/// React reducer hook section (appears in every reducer file).
/// `react_module` is hardcoded as `React` in the template (via gateway open).
#[derive(Boilerplate)]
pub(super) struct ReducerReactHookSectionRes<'a> {
    pub params_type: &'a str,
    pub camel_accessor: &'a str,
    /// Relative path to the schema module's `.res.mjs` file (e.g. `./Stdb__Schema`).
    pub schema_module_path: &'a str,
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

/// `StdbApi.res` — typed reducer/procedure records (leaf dependency, breaks cycle).
#[derive(Boilerplate)]
pub(super) struct StdbApiRes<'a> {
    pub header: AutoGenHeaderRes,
    pub reducer_fields: Vec<ReducerFieldRes<'a>>,
    pub procedure_fields: Vec<ProcedureFieldRes<'a>>,
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
    /// Pre-rendered display projection section, or empty string if unit type.
    pub display_section: &'a str,
    /// Pre-rendered `module Alias = Root__Alias` lines (replaces `open {root_module}`).
    pub sibling_opens: &'a str,
}

/// Per-reducer file (with args): `Stdb*Reducer.res`.
#[derive(Boilerplate)]
pub(super) struct ReducerWithArgsFileRes<'a> {
    pub header: AutoGenHeaderRes,
    /// Pre-rendered args record type block.
    pub args_record: &'a str,
    pub accessor: &'a str,
    /// Pre-rendered React hooks section, or empty string when async_style = Observer.
    pub react_hooks: &'a str,
    /// Pre-rendered Make functor section, or empty string when async_style = Promise.
    pub make_functor: &'a str,
    /// Pre-rendered `module Alias = Root__Alias` lines (replaces `open {root_module}`).
    pub sibling_opens: &'a str,
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
    /// Pre-rendered `module Alias = Root__Alias` lines (replaces `open {root_module}`).
    pub sibling_opens: &'a str,
}

/// Per-reducer server file: `Stdb__Reducers__X__Server.res`.
/// Typed error return via try/catch: `promise<result<unit, exn>>`.
#[derive(Boilerplate)]
pub(super) struct ReducerServerFileRes<'a> {
    pub header: AutoGenHeaderRes,
    pub has_args: bool,
    /// Dotted module path for `open` (e.g. `Reducers.Foo`), not double-underscore.
    /// Works because `sibling_opens` provides `module Reducers = {root}__Reducers`.
    pub reducer_module: &'a str,
    /// Pre-rendered `module Alias = Root__Alias` lines (replaces `open {root_module}`).
    pub sibling_opens: &'a str,
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
    pub accessor: &'a str,
    pub has_args: bool,
    /// True when the procedure returns `result<T, E>` (needs SDK→ReScript shim).
    pub is_result: bool,
    /// Pre-rendered ok type expression (non-empty only when `is_result`).
    pub ok_type: &'a str,
    /// Pre-rendered err type expression (non-empty only when `is_result`).
    pub err_type: &'a str,
    /// Pre-rendered Make functor section, or empty string when async_style = Promise.
    pub make_functor: &'a str,
    /// Pre-rendered `module Alias = Root__Alias` lines (replaces `open {root_module}`).
    pub sibling_opens: &'a str,
}

/// Make functor section for procedure files (with or without args).
/// Emitted when async_style ∈ {Observer, All}.
/// Contains: module Make = (E: Async.EFFECT_RUNTIME) => { let call ... }
#[derive(Boilerplate)]
pub(super) struct ProcedureMakeFunctorRes {
    pub has_args: bool,
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
    /// Pre-rendered `module Alias = Root__Alias` lines (replaces `open {root_module}`).
    pub sibling_opens: &'a str,
}

// ===========================================================================
// Display Layer 0: Atomic pieces for StdbDisplay.res
// ===========================================================================

/// A single match arm in an enum toString function.
/// Renders: `  | Constructor => "Constructor"`
#[derive(Boilerplate)]
pub(super) struct DisplayEnumArmRes<'a> {
    pub module_name: &'a str,
    pub constructor: &'a str,
}

/// A unit arm in a Sum enum toString function (no payload).
/// Renders: `  | Types.Module.Constructor => "Constructor"`
#[derive(Boilerplate)]
pub(super) struct DisplaySumUnitArmRes<'a> {
    pub module_name: &'a str,
    pub constructor: &'a str,
}

/// A payload arm in a Sum enum toString function.
/// Renders: `  | Types.Module.Constructor(payload) => "Constructor(" ++ payloadExpr ++ ")"`
#[derive(Boilerplate)]
pub(super) struct DisplaySumPayloadArmRes<'a> {
    pub module_name: &'a str,
    pub constructor: &'a str,
    /// Expression to convert payload to string, e.g. `payload` for string, or `excludeReasonToString(payload)`
    pub payload_expr: &'a str,
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

/// A Sum enum toString function with mixed unit/payload arms.
/// Arms are pre-rendered strings (mix of DisplaySumUnitArmRes and DisplaySumPayloadArmRes).
#[derive(Boilerplate)]
pub(super) struct DisplaySumToStringRes<'a> {
    pub fn_name: &'a str,
    pub module_name: &'a str,
    pub arms: Vec<&'a str>,
}

/// A single match arm in an enum fromString function.
/// Renders: `  | "Constructor" => Some(Constructor)`
#[derive(Boilerplate)]
pub(super) struct DisplayEnumFromStringArmRes<'a> {
    pub module_name: &'a str,
    pub constructor: &'a str,
}

/// A single enum fromString function with switch arms + catch-all.
#[derive(Boilerplate)]
pub(super) struct DisplayEnumFromStringRes<'a> {
    pub fn_name: &'a str,
    pub module_name: &'a str,
    pub arms: Vec<DisplayEnumFromStringArmRes<'a>>,
}

// ===========================================================================
// Display Layer 2: File Composition
// ===========================================================================

/// `StdbDisplay.res` — mechanical unwrappers, toString, and fromString functions.
#[derive(Boilerplate)]
pub(super) struct StdbDisplayRes<'a> {
    pub header: AutoGenHeaderRes,
    /// Pre-rendered newtype unwrapper lines.
    pub unwrappers: &'a str,
    /// Pre-rendered enum toString functions (PlainEnum).
    pub enum_to_strings: &'a str,
    /// Pre-rendered enum fromString functions (PlainEnum).
    pub enum_from_strings: &'a str,
    /// Pre-rendered sum enum toString functions (Sum types with payloads).
    pub sum_to_strings: &'a str,
    /// Pre-rendered `module Alias = Root__Alias` lines (replaces `open {root_module}`).
    pub sibling_opens: &'a str,
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
/// Renders: `{name: "proc_name", accessorName: "procName", params: {...}, returnType: {...}},`
#[derive(Boilerplate)]
pub(super) struct SchemaProcedureEntryRes<'a> {
    pub procedure_name: &'a str,
    pub accessor_name: &'a str,
    pub param_elements: Vec<SchemaProductElementRes<'a>>,
    pub return_type_expr: &'a str,
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

/// `{root}__Hooks.res` — observer-backed hooks + connection framework.
/// Provides connection context, generic `useRows`/`useCallWith`/`useCallUnit`,
/// plain subscriptions, and `mkTable` helper for `{root}__Bridge.res`.
/// Parameterized by `root_module` for sibling module references.
#[derive(Boilerplate)]
pub(super) struct StdbHooksRes<'a> {
    pub root_module: &'a str,
}

/// A single table config entry in `{root}__Bridge.res`.
/// Renders: `let configName: Hooks.tableConfig<TableModule.t> = Hooks.mkTable(...)`
#[derive(Boilerplate)]
pub(super) struct BridgeTableEntryRes<'a> {
    /// camelCase config name, e.g. `myReceipts`
    pub config_name: &'a str,
    /// Field name on `Client.db`, e.g. `myReceipts`
    pub accessor: &'a str,
    /// Full table module name, e.g. `Stdb__Tables__MyReceipts`
    pub table_module: &'a str,
}

/// `{root}__Bridge.res` — per-schema table configs for `useRows`.
#[derive(Boilerplate)]
pub(super) struct StdbBridgeRes<'a> {
    pub header: AutoGenHeaderRes,
    pub root_module: &'a str,
    pub table_entries: Vec<BridgeTableEntryRes<'a>>,
}

/// Typed event union section for table files.
/// Emitted unconditionally (regardless of async_style).
/// Contains: type event, let subscribe
#[derive(Boilerplate)]
pub(super) struct TableEventSectionRes;

/// Observer functor section for table files.
/// Emitted when async_style ∈ {Observer, All}.
/// Contains: module MakeStream with observe + observeWithCtx
#[derive(Boilerplate)]
pub(super) struct TableObserverSectionRes;

// ===========================================================================
// Table Functor: shared boilerplate for functor-style table generation
// ===========================================================================

/// `Stdb__TableFunctor.res` — TABLE module type + Make functor.
/// Generated once as a global file when `table_style = "functor"`.
/// Provides: type event, let subscribe, module MakeStream.
#[derive(Boilerplate)]
pub(super) struct TableFunctorRes {
    pub header: AutoGenHeaderRes,
    /// Pre-rendered `module Alias = Root__Alias` lines.
    pub sibling_opens: String,
    /// Whether to emit observer MakeStream section (async_style ∈ {Observer, All}).
    pub has_observer: bool,
}

/// Per-table file in functor mode: thin wrapper with `include TableFunctor.Make(...)`.
#[derive(Boilerplate)]
pub(super) struct TableFunctorFileRes<'a> {
    pub header: AutoGenHeaderRes,
    /// Pre-rendered row record type block.
    pub row_type: &'a str,
    /// Pre-rendered PK index section, or empty string if no PK.
    pub pk_section: &'a str,
    pub table_name: &'a str,
    /// Pre-rendered React hooks section, or empty string when async_style = Observer.
    pub react_hooks: &'a str,
    /// Pre-rendered display projection section, or empty string if unit type.
    pub display_section: &'a str,
    /// Pre-rendered `module Alias = Root__Alias` lines.
    pub sibling_opens: &'a str,
}

/// Make functor section for reducer files (with or without args).
/// Emitted when async_style ∈ {Observer, All}.
/// Contains: module Make = (E: Async.EFFECT_RUNTIME) => { let call ... }
#[derive(Boilerplate)]
pub(super) struct ReducerMakeFunctorRes {
    pub has_args: bool,
}

// ===========================================================================
// Display Projection: per-table `type display` + `let toDisplay`
// ===========================================================================

/// A single field assignment in `let toDisplay` body.
/// Renders: `  camelName: convertExpr,`
#[derive(Boilerplate)]
pub(super) struct DisplayProjectionFieldRes<'a> {
    pub camel_name: &'a str,
    pub convert_expr: &'a str,
}

/// A single field in the `type display` record.
/// Renders: `  fieldName: typeStr,`
#[derive(Boilerplate)]
pub(super) struct DisplayProjectionTypeFieldRes<'a> {
    pub camel_name: &'a str,
    pub type_str: &'a str,
}

/// Display projection section for a table file.
/// Contains: `type display = { ... }` + `let toDisplay = (row: t): display => { ... }`
#[derive(Boilerplate)]
pub(super) struct DisplayProjectionRes<'a> {
    pub type_fields: Vec<DisplayProjectionTypeFieldRes<'a>>,
    pub body_fields: Vec<DisplayProjectionFieldRes<'a>>,
}
