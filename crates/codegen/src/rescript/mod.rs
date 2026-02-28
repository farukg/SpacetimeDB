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

mod client;
pub(crate) mod helpers;
mod index_file;
mod react;
mod schema;
mod server_reducers;
pub(super) mod templates;
mod topo;
mod types;

use crate::util::is_reducer_invokable;
use crate::{CodegenOptions, OutputFile};

use super::Lang;
use helpers::{render_record_type, render_res_type, rescript_field_name, table_module_name, TypeRefStyle};
use templates::{
    AutoGenHeaderRes, PkIndexSectionRes, ProcedureFileRes, ReducerNoArgsFileRes, ReducerReactHookSectionRes,
    ReducerWithArgsFileRes, TableFileRes, TableReactHookSectionRes,
};

use convert_case::{Case, Casing};
use spacetimedb_schema::def::{ModuleDef, ProcedureDef, ReducerDef, TableDef};
use spacetimedb_schema::schema::TableSchema;
use std::fmt::Write;
use std::ops::Deref;

pub struct ReScript;

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

        // Pre-render row record type.
        let row_type = render_record_type(module, "t", &product_def.elements, TypeRefStyle::External);

        // Pre-render PK index section.
        let pk_section_str;
        let pk_section: &str = if let Some(pk_col) = table.primary_key {
            let (pk_field, pk_type) = &product_def.elements[pk_col.idx()];
            let pk_field_raw = pk_field.deref();
            let pk_field_camel = rescript_field_name(pk_field_raw.to_case(Case::Camel));
            let type_buf = render_res_type(module, pk_type, TypeRefStyle::External);

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

        let has_deleted_at = product_def
            .elements
            .iter()
            .any(|(field_name, _)| field_name.deref() == "deleted_at");

        let accessor = table.accessor_name.deref();

        OutputFile {
            filename: format!("tables/{}.res", table_module_name(&table.accessor_name)),
            code: TableFileRes {
                header: AutoGenHeaderRes,
                row_type: row_type.trim_end(),
                has_deleted_at,
                pk_section,
                table_name: &table.name,
                react_hooks: TableReactHookSectionRes { accessor },
            }
            .to_string(),
        }
    }

    fn generate_type_files(&self, _module: &ModuleDef, _typ: &spacetimedb_schema::def::TypeDef) -> Vec<OutputFile> {
        vec![]
    }

    /// Generates `Stdb[ReducerName]Reducer.res` — one file per reducer.
    fn generate_reducer_file(&self, module: &ModuleDef, reducer: &ReducerDef) -> OutputFile {
        if !is_reducer_invokable(reducer) {
            return OutputFile {
                filename: format!("reducers/{}.res", helpers::reducer_module_name(&reducer.name)),
                code: String::new(),
            };
        }

        let accessor = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
        let camel_accessor = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
        let elements = &reducer.params_for_generate.elements;

        if elements.is_empty() {
            OutputFile {
                filename: format!("reducers/{}.res", helpers::reducer_module_name(&reducer.name)),
                code: ReducerNoArgsFileRes {
                    header: AutoGenHeaderRes,
                    accessor: &accessor,
                    react_hooks: ReducerReactHookSectionRes {
                        params_type: "unit",
                        camel_accessor: &camel_accessor,
                    },
                }
                .to_string(),
            }
        } else {
            // Pre-render args record type.
            let args_record = render_record_type(module, "args", elements, TypeRefStyle::External);

            // Pre-render labeled params for `let call`.
            let call_params = {
                let mut buf = String::new();
                for (i, (field, ty)) in elements.iter().enumerate() {
                    let field_name = rescript_field_name(field.deref().to_case(Case::Camel));
                    let type_str = render_res_type(module, ty, TypeRefStyle::External);
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

            OutputFile {
                filename: format!("reducers/{}.res", helpers::reducer_module_name(&reducer.name)),
                code: ReducerWithArgsFileRes {
                    header: AutoGenHeaderRes,
                    args_record: args_record.trim_end(),
                    accessor: &accessor,
                    call_params: &call_params,
                    call_body_fields: call_body_fields.trim_end(),
                    react_hooks: ReducerReactHookSectionRes {
                        params_type: "args",
                        camel_accessor: &camel_accessor,
                    },
                }
                .to_string(),
            }
        }
    }

    fn generate_procedure_file(&self, module: &ModuleDef, procedure: &ProcedureDef) -> OutputFile {
        // Pre-render params record type.
        let params_record = render_record_type(
            module,
            "params",
            &procedure.params_for_generate.elements,
            TypeRefStyle::External,
        );

        // Pre-render result type expression.
        let result_type = render_res_type(module, &procedure.return_type_for_generate, TypeRefStyle::External);

        OutputFile {
            filename: format!(
                "procedures/{}.res",
                helpers::procedure_module_name(&procedure.accessor_name)
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

    /// Returns global files: StdbTypes.res, StdbSchema.res, StdbClient.res, index.res,
    /// StdbServerReducers.res, StdbReact.res, SpacetimeDBProvider.res.
    fn generate_global_files(&self, module: &ModuleDef, options: &CodegenOptions) -> Vec<OutputFile> {
        let mut files = vec![
            types::generate_types_file(module),
            schema::generate_schema_file(module, options),
            client::generate_client_file(module, options),
            index_file::generate_index_file(module, options),
            server_reducers::generate_server_reducers_file(module, options),
        ];
        files.extend(react::generate_react_file(module));
        files
    }
}
