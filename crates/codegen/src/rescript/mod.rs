//! ReScript codegen for SpacetimeDB.
//!
//! Generates `.res` and `.mjs` files from a SpacetimeDB module definition.
//! Split into submodules by concern:
//!
//! - `templates` — `#[derive(Boilerplate)]` struct definitions (3-layer composition)
//! - `helpers` — name munging, `TypeRefStyle`, record/sum/enum type renderers
//! - `topo` — Tarjan's SCC + topological sort for type emission ordering
//! - `types` — `StdbTypes.res` generator (per-type submodules)
//! - `client` — `StdbClient.res` generator (db record + connection externals)
//! - `index_file` — `index.res` generator (module aliases)
//! - `server_reducers` — `StdbServerReducers.res` generator
//! - `react` — `StdbReact.res` + `SpacetimeDBProvider.res` generators
//! - `schema` — `StdbSchema.res` generator (ReScript runtime schema)
//! - `display` — `StdbDisplay.res` generator (unwrappers + toString helpers)

mod api;
mod bridge;
mod client;
pub mod config;
mod display;
mod display_projection;
pub(crate) mod helpers;
mod hooks;
mod index_file;
mod labels;
mod react;
mod schema;
mod server_reducers;
pub(super) mod templates;
mod topo;
mod types;

use crate::util::{is_reducer_invokable, iter_reducers, iter_table_names_and_types};
use crate::{AsyncStyle, CodegenOptions, OutputFile};

use super::Lang;
use helpers::{
    procedure_module_name, reducer_module_name, render_record_type, render_res_type, rescript_field_name,
    sibling_opens, table_module_name, TypeRefStyle,
};
use templates::{
    AutoGenHeaderRes, PkIndexSectionRes, ProcedureFileRes, ProcedureMakeFunctorRes, ReducerMakeFunctorRes,
    ReducerNoArgsFileRes, ReducerReactHookSectionRes, ReducerServerFileRes, ReducerWithArgsFileRes, StdbAsyncRes,
    TableEventSectionRes, TableFileRes, TableFunctorFileRes, TableFunctorRes, TableObserverSectionRes,
    TableReactHookSectionRes,
};

use convert_case::{Case, Casing};
use spacetimedb_schema::def::{ModuleDef, ProcedureDef, ReducerDef, TableDef};
use spacetimedb_schema::schema::TableSchema;
use std::ops::Deref;
use std::path::PathBuf;

pub struct ReScript {
    pub async_style: AsyncStyle,
    pub root_module: String,
    pub output_dir_strategy: config::OutputDirStrategy,
    pub table_style: config::TableStyle,
    /// Output directory — used to read existing files for preserving human-written content (e.g. labels).
    pub out_dir: Option<PathBuf>,
}

impl Lang for ReScript {
    /// Generates `Stdb[TableName]Table.res` — one file per table.
    fn generate_table_file_from_schema(
        &self,
        module: &ModuleDef,
        table: &TableDef,
        _schema: TableSchema,
    ) -> OutputFile {
        let type_ref = table.product_type_ref;
        let product_def = module.typespace_for_generate()[type_ref].as_product().unwrap();

        let root_module = &self.root_module;
        let schema_module_path = match self.output_dir_strategy {
            config::OutputDirStrategy::Flat => format!("./{root_module}__Schema"),
            config::OutputDirStrategy::Subdirectories => format!("../{root_module}__Schema"),
        };

        // Pre-render row record type.
        let row_type = render_record_type(
            module,
            "t",
            &product_def.elements,
            TypeRefStyle::ViaGateway,
            root_module,
        );

        // Pre-render PK info (shared by PK index section + React hooks).
        let pk_field_camel_buf;
        let pk_type_buf;
        let has_pk = table.primary_key.is_some();
        let (pk_field_camel, pk_field_raw_str, pk_type_str): (&str, &str, &str) =
            if let Some(pk_col) = table.primary_key {
                let (pk_field, pk_type) = &product_def.elements[pk_col.idx()];
                pk_field_camel_buf = rescript_field_name(pk_field.deref().to_case(Case::Camel));
                pk_type_buf = render_res_type(module, pk_type, TypeRefStyle::ViaGateway, root_module);
                (&pk_field_camel_buf, pk_field.deref(), &pk_type_buf)
            } else {
                // Assignments required for borrow lifetimes even though values are empty.
                pk_field_camel_buf = String::new();
                pk_type_buf = String::new();
                (&pk_field_camel_buf, "", &pk_type_buf)
            };

        // Pre-render PK index section.
        let pk_section_str;
        let pk_section: &str = if has_pk {
            pk_section_str = PkIndexSectionRes {
                field_camel: pk_field_camel,
                field_raw: pk_field_raw_str,
                find_param_type: pk_type_str,
            }
            .to_string();
            &pk_section_str
        } else {
            ""
        };

        let accessor = table.accessor_name.deref();
        let has_display = !product_def.elements.is_empty();

        // Display projection: type display + let toDisplay.
        let display_section = display_projection::render_display_section(module, &product_def.elements, root_module);

        // Emit React hooks when async_style ∈ {Promise, All}.
        let react_hooks_str;
        let react_hooks: &str = match self.async_style {
            AsyncStyle::Promise | AsyncStyle::All => {
                // Detect if PK type is Sdk.identity (opaque JS class requiring identityIsEqual).
                let pk_is_identity = pk_type_str == "Sdk.identity";
                react_hooks_str = TableReactHookSectionRes {
                    accessor,
                    has_pk,
                    pk_type: pk_type_str,
                    pk_field_camel,
                    pk_is_identity,
                    has_display,
                    schema_module_path: &schema_module_path,
                }
                .to_string();
                &react_hooks_str
            }
            AsyncStyle::Observer => "",
        };

        match self.table_style {
            config::TableStyle::Functor => {
                // Functor mode: thin per-table file with `include TableFunctor.Make(...)`.
                // Event union, subscribe, and MakeStream come from the shared functor.
                let mut table_siblings: Vec<&str> = vec!["Sdk", "Types"];
                if !display_section.is_empty() {
                    table_siblings.push("Display");
                }
                // TableFunctor is needed for the `include`.
                table_siblings.push("TableFunctor");
                if !react_hooks.is_empty() {
                    table_siblings.push("React");
                }
                let table_opens = sibling_opens(root_module, &table_siblings);

                OutputFile {
                    filename: format!("{}.res", table_module_name(root_module, &table.accessor_name)),
                    code: TableFunctorFileRes {
                        header: AutoGenHeaderRes,
                        row_type: row_type.trim_end(),
                        pk_section,
                        table_name: &table.name,
                        react_hooks,
                        display_section: &display_section,
                        sibling_opens: &table_opens,
                    }
                    .to_string(),
                }
            }
            config::TableStyle::Inline => {
                // Inline mode (original behavior): all boilerplate inlined per table file.
                // Always emit the typed event union + subscribe.
                let event_section = TableEventSectionRes.to_string();

                // Emit observer functor when async_style ∈ {Observer, All}.
                let observer_section_str;
                let observer_section: &str = match self.async_style {
                    AsyncStyle::Observer | AsyncStyle::All => {
                        observer_section_str = TableObserverSectionRes.to_string();
                        &observer_section_str
                    }
                    AsyncStyle::Promise => "",
                };

                let mut table_siblings: Vec<&str> = vec!["Sdk", "Types"];
                if !display_section.is_empty() {
                    table_siblings.push("Display");
                }
                if !observer_section.is_empty() {
                    table_siblings.push("Async");
                }
                if !react_hooks.is_empty() {
                    table_siblings.push("React");
                }
                let table_opens = sibling_opens(root_module, &table_siblings);

                OutputFile {
                    filename: format!("{}.res", table_module_name(root_module, &table.accessor_name)),
                    code: TableFileRes {
                        header: AutoGenHeaderRes,
                        row_type: row_type.trim_end(),
                        pk_section,
                        table_name: &table.name,
                        event_section: &event_section,
                        observer_section,
                        react_hooks,
                        display_section: &display_section,
                        sibling_opens: &table_opens,
                    }
                    .to_string(),
                }
            }
        }
    }

    fn generate_type_files(&self, _module: &ModuleDef, _typ: &spacetimedb_schema::def::TypeDef) -> Vec<OutputFile> {
        vec![]
    }

    /// Generates `Stdb[ReducerName]Reducer.res` — one file per reducer.
    fn generate_reducer_file(&self, module: &ModuleDef, reducer: &ReducerDef) -> OutputFile {
        let root_module = &self.root_module;
        let schema_module_path = match self.output_dir_strategy {
            config::OutputDirStrategy::Flat => format!("./{root_module}__Schema"),
            config::OutputDirStrategy::Subdirectories => format!("../{root_module}__Schema"),
        };

        let accessor = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
        let camel_accessor = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
        let elements = &reducer.params_for_generate.elements;

        // Pre-render Make functor when async_style ∈ {Observer, All}.
        let make_functor_no_args_str;
        let make_functor_with_args_str;

        if elements.is_empty() {
            // Pre-render React hooks when async_style ∈ {Promise, All}.
            let react_hooks_str;
            let react_hooks: &str = match self.async_style {
                AsyncStyle::Promise | AsyncStyle::All => {
                    react_hooks_str = ReducerReactHookSectionRes {
                        params_type: "unit",
                        camel_accessor: &camel_accessor,
                        schema_module_path: &schema_module_path,
                    }
                    .to_string();
                    &react_hooks_str
                }
                AsyncStyle::Observer => "",
            };

            let make_functor: &str = match self.async_style {
                AsyncStyle::Observer | AsyncStyle::All => {
                    make_functor_no_args_str = ReducerMakeFunctorRes { has_args: false }.to_string();
                    &make_functor_no_args_str
                }
                AsyncStyle::Promise => "",
            };

            // No-args reducer: needs Sdk (connection + opaque reducers).
            // Async is needed when make_functor is non-empty (observer mode).
            // React is needed when react_hooks is non-empty.
            let mut no_args_siblings: Vec<&str> = vec!["Sdk"];
            if !make_functor.is_empty() {
                no_args_siblings.push("Async");
            }
            if !react_hooks.is_empty() {
                no_args_siblings.push("React");
            }
            let no_args_opens = sibling_opens(root_module, &no_args_siblings);

            OutputFile {
                filename: format!("{}.res", reducer_module_name(root_module, &reducer.name)),
                code: ReducerNoArgsFileRes {
                    header: AutoGenHeaderRes,
                    accessor: &accessor,
                    react_hooks,
                    make_functor,
                    sibling_opens: &no_args_opens,
                }
                .to_string(),
            }
        } else {
            // Pre-render args record type.
            let args_record = render_record_type(module, "args", elements, TypeRefStyle::ViaGateway, root_module);

            // Pre-render React hooks when async_style ∈ {Promise, All}.
            let react_hooks_str;
            let react_hooks: &str = match self.async_style {
                AsyncStyle::Promise | AsyncStyle::All => {
                    react_hooks_str = ReducerReactHookSectionRes {
                        params_type: "args",
                        camel_accessor: &camel_accessor,
                        schema_module_path: &schema_module_path,
                    }
                    .to_string();
                    &react_hooks_str
                }
                AsyncStyle::Observer => "",
            };

            let make_functor: &str = match self.async_style {
                AsyncStyle::Observer | AsyncStyle::All => {
                    make_functor_with_args_str = ReducerMakeFunctorRes { has_args: true }.to_string();
                    &make_functor_with_args_str
                }
                AsyncStyle::Promise => "",
            };

            // With-args reducer: needs Sdk (connection + opaque reducers), Types (args field types).
            // Async is needed when make_functor is non-empty (observer mode).
            // React is needed when react_hooks is non-empty.
            let mut with_args_siblings: Vec<&str> = vec!["Sdk", "Types"];
            if !make_functor.is_empty() {
                with_args_siblings.push("Async");
            }
            if !react_hooks.is_empty() {
                with_args_siblings.push("React");
            }
            let with_args_opens = sibling_opens(root_module, &with_args_siblings);

            OutputFile {
                filename: format!("{}.res", reducer_module_name(root_module, &reducer.name)),
                code: ReducerWithArgsFileRes {
                    header: AutoGenHeaderRes,
                    args_record: args_record.trim_end(),
                    accessor: &accessor,
                    react_hooks,
                    make_functor,
                    sibling_opens: &with_args_opens,
                }
                .to_string(),
            }
        }
    }

    fn generate_procedure_file(&self, module: &ModuleDef, procedure: &ProcedureDef) -> OutputFile {
        let root_module = &self.root_module;

        let accessor = rescript_field_name(procedure.accessor_name.deref().to_case(Case::Camel));
        let elements = &procedure.params_for_generate.elements;
        let has_args = !elements.is_empty();

        // Pre-render params record type.
        let params_record = render_record_type(module, "params", elements, TypeRefStyle::ViaGateway, root_module);

        // Pre-render result type expression.
        let result_type = render_res_type(
            module,
            &procedure.return_type_for_generate,
            TypeRefStyle::ViaGateway,
            root_module,
        );

        // SDK BSATN returns {ok, err} but ReScript result uses {TAG, _0} — need shim.
        // Extract ok/err type strings for Sdk.sdkResult<ok, err> in the external.
        let ok_type_str;
        let err_type_str;
        let (is_result, ok_type, err_type): (bool, &str, &str) = match &procedure.return_type_for_generate {
            spacetimedb_schema::type_for_generate::AlgebraicTypeUse::Result { ok_ty, err_ty } => {
                ok_type_str = render_res_type(module, ok_ty, TypeRefStyle::ViaGateway, root_module);
                err_type_str = if matches!(
                    err_ty.as_ref(),
                    spacetimedb_schema::type_for_generate::AlgebraicTypeUse::String
                ) {
                    format!(
                        "option<{}>",
                        render_res_type(module, err_ty, TypeRefStyle::ViaGateway, root_module)
                    )
                } else {
                    render_res_type(module, err_ty, TypeRefStyle::ViaGateway, root_module)
                };
                (true, ok_type_str.as_str(), err_type_str.as_str())
            }
            _ => (false, "", ""),
        };

        // Pre-render Make functor when async_style ∈ {Observer, All}.
        let make_functor_str;
        let make_functor: &str = match self.async_style {
            AsyncStyle::Observer | AsyncStyle::All => {
                make_functor_str = ProcedureMakeFunctorRes { has_args }.to_string();
                &make_functor_str
            }
            AsyncStyle::Promise => "",
        };

        // Build sibling opens: Sdk, Client, Types (+ Async when functor present).
        // Procedure file: needs Sdk (connection + opaque procedures), Types (params/response field types).
        let mut siblings: Vec<&str> = vec!["Sdk", "Types"];
        if !make_functor.is_empty() {
            siblings.push("Async");
        }
        let sibling_opens = sibling_opens(root_module, &siblings);

        OutputFile {
            filename: format!("{}.res", procedure_module_name(root_module, &procedure.accessor_name)),
            code: ProcedureFileRes {
                header: AutoGenHeaderRes,
                params_record: params_record.trim_end(),
                result_type: &result_type,
                procedure_name: &procedure.name,
                accessor: &accessor,
                has_args,
                is_result,
                ok_type,
                err_type,
                make_functor,
                sibling_opens: &sibling_opens,
            }
            .to_string(),
        }
    }

    /// Returns global files:
    /// - {root}.res (root gateway)
    /// - {root}__Types.res, {root}__Schema.res, {root}__Client.res
    /// - {root}__Tables.res (table gateway), {root}__Reducers.res (reducer gateway)
    /// - {root}__React.res, {root}__Display.res
    /// - {root}__ServerReducers.res
    /// - {root}__Sdk.res (re-export shim)
    /// - {root}__Async.res (when async_style ≠ Promise)
    /// - {root}__Procedures.res (when procedures exist)
    fn generate_global_files(&self, module: &ModuleDef, options: &CodegenOptions) -> Vec<OutputFile> {
        let root_module = &self.root_module;

        // Collect table row type refs so Types can skip them (they live in per-table files).
        let table_row_type_refs: std::collections::HashSet<spacetimedb_lib::sats::AlgebraicTypeRef> =
            iter_table_names_and_types(module, options.visibility)
                .map(|(_, _, type_ref)| type_ref)
                .collect();

        let mut files = vec![
            types::generate_types_file(module, root_module, &table_row_type_refs),
            schema::generate_schema_file(module, options, root_module),
            client::generate_client_file(module, options, root_module),
            api::generate_api_file(module, options, root_module),
            server_reducers::generate_server_reducers_file(module, options, root_module),
            display::generate_display_file(module, root_module),
        ];
        // React/Provider: only emit when async_style ∈ {Promise, All}.
        // Observer-only mode must never import @spacetimedb/rescript/react
        // (which calls React.createContext at module init — crashing server-side Node).
        if self.async_style != AsyncStyle::Observer {
            files.extend(react::generate_react_file(module, root_module));
        }
        // Namespace gateways: root, tables, reducers, procedures.
        files.extend(index_file::generate_gateway_files(
            module,
            options,
            root_module,
            self.async_style,
        ));
        // Re-export shim: {root_module}__Sdk.res includes the runtime Stdb__Sdk module.
        // Skip when root_module == "Stdb" — the runtime package already provides Stdb__Sdk.res;
        // generating a same-named file creates a circular self-include.
        if root_module != "Stdb" {
            files.push(OutputFile {
                filename: format!("{root_module}__Sdk.res"),
                code: format!("{header}\ninclude Stdb__Sdk\n", header = AutoGenHeaderRes,),
            });
        }
        if self.async_style != AsyncStyle::Promise {
            files.push(OutputFile {
                filename: format!("{root_module}__Async.res"),
                code: StdbAsyncRes.to_string(),
            });
            files.push(hooks::generate_hooks_file(root_module));
            files.push(bridge::generate_bridge_file(module, options, root_module));
        }
        // TableFunctor: shared module type + Make functor (only in functor mode).
        if self.table_style == config::TableStyle::Functor {
            let has_observer = self.async_style != AsyncStyle::Promise;
            let mut functor_siblings: Vec<&str> = vec!["Sdk"];
            if has_observer {
                functor_siblings.push("Async");
            }
            let functor_opens = sibling_opens(root_module, &functor_siblings);
            files.push(OutputFile {
                filename: format!("{root_module}__TableFunctor.res"),
                code: TableFunctorRes {
                    header: AutoGenHeaderRes,
                    sibling_opens: functor_opens,
                    has_observer,
                }
                .to_string(),
            });
        }
        // Labels stub: per-PlainEnum translation functions.
        // Read existing labels file to preserve human-written translation strings.
        let existing_labels = self.out_dir.as_ref().and_then(|dir| {
            let labels_path = dir.join(format!("{root_module}__Labels.res"));
            std::fs::read_to_string(&labels_path).ok()
        });
        let labels_file = labels::generate_labels_file(module, root_module, existing_labels.as_deref());
        if !labels_file.code.is_empty() {
            files.push(labels_file);
        }
        // Per-reducer server files: typed error return via try/catch.
        for reducer in iter_reducers(module, options.visibility) {
            if !is_reducer_invokable(reducer) {
                continue;
            }
            let has_args = !reducer.params_for_generate.elements.is_empty();
            let reducer_mod_dotted = format!("Reducers.{}", reducer.name.deref().to_case(Case::Pascal));
            let reducer_mod = reducer_module_name(root_module, &reducer.name);
            // Server file: needs Sdk (connection + opaque reducers), Reducers (for `open Reducers.Foo`).
            let server_opens = sibling_opens(root_module, &["Sdk", "Reducers"]);
            files.push(OutputFile {
                filename: format!("{reducer_mod}__Server.res"),
                code: ReducerServerFileRes {
                    header: AutoGenHeaderRes,
                    has_args,
                    reducer_module: &reducer_mod_dotted,
                    sibling_opens: &server_opens,
                }
                .to_string(),
            });
        }
        files
    }
}
