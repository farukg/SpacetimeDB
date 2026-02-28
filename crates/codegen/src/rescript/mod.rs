//! ReScript codegen for SpacetimeDB.
//!
//! Generates `.res` and `.mjs` files from a SpacetimeDB module definition.
//! Split into submodules by concern:
//!
//! - `helpers` — name munging, `TypeRefStyle`, record/sum/enum type writers
//! - `topo` — Tarjan's SCC + topological sort for type emission ordering
//! - `types` — `StdbTypes.res` generator (per-type submodules)
//! - `client` — `StdbClient.res` generator (db record + connection externals)
//! - `index_file` — `index.res` generator (module aliases)
//! - `server_reducers` — `StdbServerReducers.res` generator
//! - `react` — `StdbReact.res` + `SpacetimeDBProvider.res` generators
//! - `schema` — `StdbSchema.mjs` generator (JS runtime schema)

mod client;
pub(crate) mod helpers;
mod index_file;
mod react;
mod schema;
mod server_reducers;
mod topo;
mod types;

use crate::util::{is_reducer_invokable, print_auto_generated_file_comment};
use crate::{CodegenOptions, OutputFile};

use super::code_indenter::CodeIndenter;
use super::Lang;
use helpers::{rescript_field_name, table_module_name, write_record_type, write_res_type, TypeRefStyle};

use convert_case::{Case, Casing};
use spacetimedb_schema::def::{ModuleDef, ProcedureDef, ReducerDef, TableDef};
use spacetimedb_schema::schema::TableSchema;
use std::ops::Deref;

const INDENT: &str = "  ";

pub struct ReScript;

impl Lang for ReScript {
    /// Generates `Stdb[TableName]Table.res` — one file per table.
    ///
    /// Contains:
    /// - Row record type (`type t = { ... }`)
    /// - Opaque table handle type (`type handle`)
    /// - `@send iter` binding
    /// - `@send onInsert / onUpdate / onDelete` bindings
    /// - `isAlive` helper (if table has `deleted_at` field)
    /// - PK index type + `@get` accessor + `@send find` (if table has a primary key)
    fn generate_table_file_from_schema(
        &self,
        module: &ModuleDef,
        table: &TableDef,
        _schema: TableSchema,
    ) -> OutputFile {
        let mut output = CodeIndenter::new(String::new(), INDENT);
        let out = &mut output;

        print_auto_generated_file_comment(out);
        writeln!(out, "");

        let type_ref = table.product_type_ref;
        let product_def = module.typespace_for_generate()[type_ref].as_product().unwrap();

        // Row record type
        write_record_type(module, out, "t", &product_def.elements, TypeRefStyle::External);

        // Opaque table handle type
        writeln!(out, "// Opaque table handle — obtained from StdbClient.db");
        writeln!(out, "type handle");
        writeln!(out, "");

        // iter: handle → Iterator.t<t>
        writeln!(out, "@send external iter: handle => Iterator.t<t> = \"iter\"");

        // onInsert / onUpdate / onDelete
        writeln!(
            out,
            "@send external onInsert: (handle, (StdbTypes.eventCtx, t) => unit) => unit = \"onInsert\""
        );
        writeln!(
            out,
            "@send external onUpdate: (handle, (StdbTypes.eventCtx, t, t) => unit) => unit = \"onUpdate\""
        );
        writeln!(
            out,
            "@send external onDelete: (handle, (StdbTypes.eventCtx, t) => unit) => unit = \"onDelete\""
        );
        writeln!(out, "");

        // isAlive: soft-delete pattern
        let has_deleted_at = product_def
            .elements
            .iter()
            .any(|(field_name, _)| field_name.deref() == "deleted_at");

        if has_deleted_at {
            writeln!(out, "let isAlive = (row: t) => row.deletedAt->Option.isNone");
            writeln!(out, "");
        }

        // PK index: type + @get accessor + @send find
        if let Some(pk_col) = table.primary_key {
            let (pk_field, pk_type) = &product_def.elements[pk_col.idx()];
            let pk_field_raw = pk_field.deref(); // snake_case — runtime SSOT
            let pk_field_camel = rescript_field_name(pk_field_raw.to_case(Case::Camel));

            writeln!(out, "// PK index");
            writeln!(out, "type {pk_field_camel}Index");
            // @get string = raw field name (snake_case) — SDK attaches index via idxDef.name
            writeln!(
                out,
                "@get external {pk_field_camel}: handle => {pk_field_camel}Index = \"{pk_field_raw}\""
            );
            write!(out, "@send external find: ({pk_field_camel}Index, ");
            write_res_type(module, out, pk_type, TypeRefStyle::External);
            writeln!(out, ") => Nullable.t<t> = \"find\"");
            writeln!(out, "");
        }

        // Table name constant
        writeln!(out, "let tableName = \"{}\"", table.name);

        // React hook bindings — typed via phantom StdbReact.query<t>
        writeln!(out, "");
        writeln!(out, "// React hook — typed query binding");
        let accessor = table.accessor_name.deref();
        writeln!(out, "@module(\"../StdbSchema.mjs\") @val");
        writeln!(out, "external query: StdbReact.query<t> = \"tables.{accessor}\"");
        writeln!(out, "");
        writeln!(out, "let useRows = () => StdbReact.useTable(query)");
        writeln!(out, "let useRowsWith = (cbs) => StdbReact.useTableWith(query, cbs)");

        OutputFile {
            filename: format!("tables/{}.res", table_module_name(&table.accessor_name)),
            code: output.into_inner(),
        }
    }

    fn generate_type_files(&self, _module: &ModuleDef, _typ: &spacetimedb_schema::def::TypeDef) -> Vec<OutputFile> {
        vec![]
    }

    /// Generates `Stdb[ReducerName]Reducer.res` — one file per reducer.
    ///
    /// Contains:
    /// - Args record type (`type args = { ... }`) — omitted if reducer has no params
    /// - `@send` binding on `StdbTypes.reducers`
    /// - Typed helper function
    fn generate_reducer_file(&self, module: &ModuleDef, reducer: &ReducerDef) -> OutputFile {
        // Skip non-invokable lifecycle reducers (init, update, etc.)
        if !is_reducer_invokable(reducer) {
            return OutputFile {
                filename: format!("reducers/{}.res", helpers::reducer_module_name(&reducer.name)),
                code: String::new(),
            };
        }

        let mut output = CodeIndenter::new(String::new(), INDENT);
        let out = &mut output;

        print_auto_generated_file_comment(out);
        writeln!(out, "");

        let accessor = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
        let elements = &reducer.params_for_generate.elements;

        if elements.is_empty() {
            // No-arg reducer: @send binding + unit helper
            writeln!(
                out,
                "@send external {accessor}: StdbTypes.reducers => promise<unit> = \"{accessor}\""
            );
            writeln!(out, "");
            writeln!(out, "let call = (conn: StdbTypes.connection) =>");
            writeln!(out, "  conn->StdbClient.reducers->{accessor}");
        } else {
            // Args record type
            write_record_type(module, out, "args", elements, TypeRefStyle::External);

            // @send binding
            writeln!(
                out,
                "@send external {accessor}: (StdbTypes.reducers, args) => promise<unit> = \"{accessor}\""
            );
            writeln!(out, "");

            // Typed helper — constructs the `args` record and calls the @send binding.
            write!(out, "let call = (conn: StdbTypes.connection, ");
            for (i, (field, ty)) in elements.iter().enumerate() {
                let field_name = rescript_field_name(field.deref().to_case(Case::Camel));
                write!(out, "~{field_name}: ");
                write_res_type(module, out, ty, TypeRefStyle::External);
                if i < elements.len() - 1 {
                    write!(out, ", ");
                }
            }
            writeln!(out, ") =>");
            out.indent(1);
            writeln!(out, "conn->StdbClient.reducers->{accessor}({{");
            out.indent(1);
            for (field, _ty) in elements.iter() {
                let camel = rescript_field_name(field.deref().to_case(Case::Camel));
                writeln!(out, "{camel}: {camel},");
            }
            out.dedent(1);
            writeln!(out, "}})");
            out.dedent(1);
        }

        // React hook binding — typed reducer caller
        writeln!(out, "");
        writeln!(out, "// React hook — typed reducer binding");
        let camel_accessor = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
        let params_type = if elements.is_empty() { "unit" } else { "args" };
        writeln!(out, "@module(\"../StdbSchema.mjs\") @val");
        writeln!(
            out,
            "external reducerDef: StdbReact.reducerDef<{params_type}> = \"reducers.{camel_accessor}\""
        );
        writeln!(out, "");
        writeln!(out, "let useCall = () => StdbReact.useReducer(reducerDef)");

        OutputFile {
            filename: format!("reducers/{}.res", helpers::reducer_module_name(&reducer.name)),
            code: output.into_inner(),
        }
    }

    fn generate_procedure_file(&self, module: &ModuleDef, procedure: &ProcedureDef) -> OutputFile {
        let mut output = CodeIndenter::new(String::new(), INDENT);
        let out = &mut output;

        print_auto_generated_file_comment(out);
        writeln!(out, "");

        write_record_type(
            module,
            out,
            "params",
            &procedure.params_for_generate.elements,
            TypeRefStyle::External,
        );
        writeln!(out, "");
        write!(out, "type result = ");
        write_res_type(module, out, &procedure.return_type_for_generate, TypeRefStyle::External);
        writeln!(out, "");
        writeln!(out, "let procedureName = \"{}\"", procedure.name);

        OutputFile {
            filename: format!(
                "procedures/{}.res",
                helpers::procedure_module_name(&procedure.accessor_name)
            ),
            code: output.into_inner(),
        }
    }

    /// Returns global files: StdbTypes.res, StdbSchema.mjs, StdbClient.res, index.res,
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
