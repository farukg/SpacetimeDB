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

mod client;
mod display;
pub(crate) mod helpers;
mod index_file;
mod react;
mod schema;
mod server_reducers;
pub(super) mod templates;
mod topo;
mod types;

use crate::util::is_reducer_invokable;
use crate::{AsyncStyle, CodegenOptions, OutputFile};

use super::Lang;
use helpers::{
    procedure_module_name, reducer_module_name, render_record_type, render_res_type, rescript_field_name,
    table_module_name, TypeRefStyle,
};
use templates::{
    AutoGenHeaderRes, PkIndexSectionRes, ProcedureFileRes, ReducerMakeFunctorRes, ReducerNoArgsFileRes,
    ReducerReactHookSectionRes, ReducerWithArgsFileRes, StdbAsyncRes, TableEventSectionRes, TableFileRes,
    TableObserverSectionRes, TableReactHookSectionRes,
};

use convert_case::{Case, Casing};
use spacetimedb_schema::def::{ModuleDef, ProcedureDef, ReducerDef, TableDef};
use spacetimedb_schema::schema::TableSchema;
use std::fmt::Write;
use std::ops::Deref;

pub struct ReScript {
    pub async_style: AsyncStyle,
    pub root_module: String,
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
        let sdk_module = format!("{root_module}__Sdk");

        // Pre-render row record type.
        let row_type = render_record_type(module, "t", &product_def.elements, TypeRefStyle::External, root_module);

        // Pre-render PK index section.
        let pk_section_str;
        let pk_section: &str = if let Some(pk_col) = table.primary_key {
            let (pk_field, pk_type) = &product_def.elements[pk_col.idx()];
            let pk_field_raw = pk_field.deref();
            let pk_field_camel = rescript_field_name(pk_field_raw.to_case(Case::Camel));
            let type_buf = render_res_type(module, pk_type, TypeRefStyle::External, root_module);

            pk_section_str = PkIndexSectionRes {
                field_camel: &pk_field_camel,
                field_raw: pk_field_raw,
                find_param_type: &type_buf,
            }
            .to_string();
            &pk_section_str
        } else {
            ""
        };

        let accessor = table.accessor_name.deref();

        // Always emit the typed event union + subscribe.
        let event_section = TableEventSectionRes {
            sdk_module: &sdk_module,
        }
        .to_string();

        // Emit observer functor when async_style ∈ {Observer, All}.
        let observer_section_str;
        let observer_section: &str = match self.async_style {
            AsyncStyle::Observer | AsyncStyle::All => {
                observer_section_str = TableObserverSectionRes {
                    sdk_module: &sdk_module,
                }
                .to_string();
                &observer_section_str
            }
            AsyncStyle::Promise => "",
        };

        // Emit React hooks when async_style ∈ {Promise, All}.
        let react_hooks_str;
        let react_hooks: &str = match self.async_style {
            AsyncStyle::Promise | AsyncStyle::All => {
                react_hooks_str = TableReactHookSectionRes { accessor }.to_string();
                &react_hooks_str
            }
            AsyncStyle::Observer => "",
        };

        OutputFile {
            filename: format!("tables/{}.res", table_module_name(root_module, &table.accessor_name)),
            code: TableFileRes {
                header: AutoGenHeaderRes,
                row_type: row_type.trim_end(),
                pk_section,
                table_name: &table.name,
                event_section: &event_section,
                observer_section,
                react_hooks,
                sdk_module: &sdk_module,
            }
            .to_string(),
        }
    }

    fn generate_type_files(&self, _module: &ModuleDef, _typ: &spacetimedb_schema::def::TypeDef) -> Vec<OutputFile> {
        vec![]
    }

    /// Generates `Stdb[ReducerName]Reducer.res` — one file per reducer.
    fn generate_reducer_file(&self, module: &ModuleDef, reducer: &ReducerDef) -> OutputFile {
        let root_module = &self.root_module;
        let sdk_module = format!("{root_module}__Sdk");

        if !is_reducer_invokable(reducer) {
            return OutputFile {
                filename: format!("reducers/{}.res", reducer_module_name(root_module, &reducer.name)),
                code: String::new(),
            };
        }

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
                    }
                    .to_string();
                    &react_hooks_str
                }
                AsyncStyle::Observer => "",
            };

            let make_functor: &str = match self.async_style {
                AsyncStyle::Observer | AsyncStyle::All => {
                    make_functor_no_args_str = ReducerMakeFunctorRes {
                        accessor: &accessor,
                        has_args: false,
                        sdk_module: &sdk_module,
                    }
                    .to_string();
                    &make_functor_no_args_str
                }
                AsyncStyle::Promise => "",
            };

            OutputFile {
                filename: format!("reducers/{}.res", reducer_module_name(root_module, &reducer.name)),
                code: ReducerNoArgsFileRes {
                    header: AutoGenHeaderRes,
                    accessor: &accessor,
                    react_hooks,
                    make_functor,
                    sdk_module: &sdk_module,
                }
                .to_string(),
            }
        } else {
            // Pre-render args record type.
            let args_record = render_record_type(module, "args", elements, TypeRefStyle::External, root_module);

            // Pre-render labeled params for `let call`.
            let call_params = {
                let mut buf = String::new();
                for (i, (field, ty)) in elements.iter().enumerate() {
                    let field_name = rescript_field_name(field.deref().to_case(Case::Camel));
                    let type_str = render_res_type(module, ty, TypeRefStyle::External, root_module);
                    write!(buf, "~{field_name}: {type_str}").unwrap();
                    if i < elements.len() - 1 {
                        buf.push_str(", ");
                    }
                }
                buf
            };

            // Pre-render record construction fields.
            let call_body_fields = {
                let mut buf = String::new();
                for (field, _ty) in elements.iter() {
                    let camel = rescript_field_name(field.deref().to_case(Case::Camel));
                    writeln!(buf, "    {camel}: {camel},").unwrap();
                }
                buf
            };

            // Pre-render React hooks when async_style ∈ {Promise, All}.
            let react_hooks_str;
            let react_hooks: &str = match self.async_style {
                AsyncStyle::Promise | AsyncStyle::All => {
                    react_hooks_str = ReducerReactHookSectionRes {
                        params_type: "args",
                        camel_accessor: &camel_accessor,
                    }
                    .to_string();
                    &react_hooks_str
                }
                AsyncStyle::Observer => "",
            };

            let make_functor: &str = match self.async_style {
                AsyncStyle::Observer | AsyncStyle::All => {
                    make_functor_with_args_str = ReducerMakeFunctorRes {
                        accessor: &accessor,
                        has_args: true,
                        sdk_module: &sdk_module,
                    }
                    .to_string();
                    &make_functor_with_args_str
                }
                AsyncStyle::Promise => "",
            };

            OutputFile {
                filename: format!("reducers/{}.res", reducer_module_name(root_module, &reducer.name)),
                code: ReducerWithArgsFileRes {
                    header: AutoGenHeaderRes,
                    args_record: args_record.trim_end(),
                    accessor: &accessor,
                    call_params: &call_params,
                    call_body_fields: call_body_fields.trim_end(),
                    react_hooks,
                    make_functor,
                    sdk_module: &sdk_module,
                }
                .to_string(),
            }
        }
    }

    fn generate_procedure_file(&self, module: &ModuleDef, procedure: &ProcedureDef) -> OutputFile {
        let root_module = &self.root_module;

        // Pre-render params record type.
        let params_record = render_record_type(
            module,
            "params",
            &procedure.params_for_generate.elements,
            TypeRefStyle::External,
            root_module,
        );

        // Pre-render result type expression.
        let result_type = render_res_type(
            module,
            &procedure.return_type_for_generate,
            TypeRefStyle::External,
            root_module,
        );

        OutputFile {
            filename: format!(
                "procedures/{}.res",
                procedure_module_name(root_module, &procedure.accessor_name)
            ),
            code: ProcedureFileRes {
                header: AutoGenHeaderRes,
                params_record: params_record.trim_end(),
                result_type: &result_type,
                procedure_name: &procedure.name,
            }
            .to_string(),
        }
    }

    /// Returns global files: {root_module}__Types.res, {root_module}__Schema.res,
    /// {root_module}__Client.res, index.res, {root_module}__ServerReducers.res,
    /// {root_module}__React.res, {root_module}__Provider.res, {root_module}__Display.res,
    /// {root_module}__Sdk.res (re-export shim).
    /// When async_style ∈ {Observer, All}, also emits {root_module}__Async.res.
    fn generate_global_files(&self, module: &ModuleDef, options: &CodegenOptions) -> Vec<OutputFile> {
        let root_module = &self.root_module;
        let mut files = vec![
            types::generate_types_file(module, root_module),
            schema::generate_schema_file(module, options, root_module),
            client::generate_client_file(module, options, root_module),
            index_file::generate_index_file(module, options, root_module),
            server_reducers::generate_server_reducers_file(module, options, root_module),
            display::generate_display_file(module, root_module),
        ];
        files.extend(react::generate_react_file(module, root_module));
        // Re-export shim: {root_module}__Sdk.res includes the runtime Stdb__Sdk module.
        // When root_module == "Stdb" this is a self-include (harmless identity).
        files.push(OutputFile {
            filename: format!("{root_module}__Sdk.res"),
            code: format!("{header}\ninclude Stdb__Sdk\n", header = AutoGenHeaderRes,),
        });
        if self.async_style != AsyncStyle::Promise {
            files.push(OutputFile {
                filename: format!("{root_module}__Async.res"),
                code: StdbAsyncRes.to_string(),
            });
        }
        files
    }
}
