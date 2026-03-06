//! StdbApi.res generation — typed reducer/procedure records.
//!
//! This is a **leaf module**: reducer/procedure files never import it.
//! Consumer code uses `Api.reducers` / `Api.procedures` for compile-time-safe
//! access to connection methods. If a reducer or procedure is removed from
//! the server and codegen is re-run, any call site referencing the missing
//! field fails to compile.

use super::helpers::{procedure_module_name, reducer_module_name, rescript_field_name};
use super::templates::{AutoGenHeaderRes, ProcedureFieldRes, ReducerFieldRes, StdbApiRes};
use crate::util::{is_reducer_invokable, iter_procedures, iter_reducers};
use crate::{CodegenOptions, OutputFile};

use convert_case::{Case, Casing};
use spacetimedb_schema::def::ModuleDef;
use std::ops::Deref;

/// Intermediate owned data for a reducer field.
struct ReducerFieldData {
    /// Raw camelCase accessor (JS runtime key).
    accessor: String,
    /// Escaped ReScript field name.
    camel: String,
    has_args: bool,
    /// Module-qualified args type, e.g. `Stdb__Reducers__Foo.args`.
    args_type: String,
}

/// Intermediate owned data for a procedure field.
struct ProcedureFieldData {
    /// Raw camelCase accessor (JS runtime key).
    accessor: String,
    /// Escaped ReScript field name.
    camel: String,
    has_args: bool,
    /// Module-qualified params type, e.g. `Stdb__Procedures__Foo.params`.
    params_type: String,
    /// Module-qualified response type, e.g. `Stdb__Procedures__Foo.response`.
    response_type: String,
}

/// Generates `StdbApi.res`.
pub(super) fn generate_api_file(module: &ModuleDef, options: &CodegenOptions, root_module: &str) -> OutputFile {
    let sdk_module = format!("{root_module}__Sdk");

    // --- Reducer fields ---
    let reducer_data: Vec<ReducerFieldData> = iter_reducers(module, options.visibility)
        .filter(|r| is_reducer_invokable(r))
        .map(|reducer| {
            let raw_camel = reducer.accessor_name.deref().to_case(Case::Camel);
            let camel = rescript_field_name(raw_camel.clone());
            let has_args = !reducer.params_for_generate.elements.is_empty();
            let reducer_mod = reducer_module_name(root_module, &reducer.name);
            let args_type = if has_args {
                format!("{reducer_mod}.args")
            } else {
                String::new()
            };
            ReducerFieldData {
                accessor: raw_camel,
                camel,
                has_args,
                args_type,
            }
        })
        .collect();

    // --- Procedure fields ---
    let procedure_data: Vec<ProcedureFieldData> = iter_procedures(module, options.visibility)
        .map(|procedure| {
            let raw_camel = procedure.accessor_name.deref().to_case(Case::Camel);
            let camel = rescript_field_name(raw_camel.clone());
            let has_args = !procedure.params_for_generate.elements.is_empty();
            let proc_mod = procedure_module_name(root_module, &procedure.accessor_name);
            let params_type = if has_args {
                format!("{proc_mod}.params")
            } else {
                String::new()
            };
            let response_type = format!("{proc_mod}.response");
            ProcedureFieldData {
                accessor: raw_camel,
                camel,
                has_args,
                params_type,
                response_type,
            }
        })
        .collect();

    // --- Build template structs ---
    let reducer_fields: Vec<ReducerFieldRes> = reducer_data
        .iter()
        .map(|f| ReducerFieldRes {
            accessor: &f.accessor,
            camel: &f.camel,
            has_args: f.has_args,
            args_type: &f.args_type,
        })
        .collect();

    let procedure_fields: Vec<ProcedureFieldRes> = procedure_data
        .iter()
        .map(|f| ProcedureFieldRes {
            accessor: &f.accessor,
            camel: &f.camel,
            has_args: f.has_args,
            params_type: &f.params_type,
            response_type: &f.response_type,
        })
        .collect();

    OutputFile {
        filename: format!("{root_module}__Api.res"),
        code: StdbApiRes {
            header: AutoGenHeaderRes,
            reducer_fields,
            procedure_fields,
            sdk_module: &sdk_module,
        }
        .to_string(),
    }
}
