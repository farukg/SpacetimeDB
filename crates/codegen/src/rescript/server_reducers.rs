//! Server-side reducer wrappers codegen — generates `StdbServerReducers.res`.

use super::helpers::rescript_field_name;
use super::templates::{
    AutoGenHeaderRes, ServerReducerTypeFieldRes, ServerReducerValueFieldRes, ServerReducerWrapperRes,
    StdbServerReducersRes,
};
use crate::util::{is_reducer_invokable, iter_reducers};
use crate::{CodegenOptions, OutputFile};

use convert_case::{Case, Casing};
use spacetimedb_schema::def::ModuleDef;
use std::ops::Deref;

/// Intermediate owned data for a reducer.
struct ReducerData {
    name_camel: String,
    module: String,
    has_args: bool,
}

pub(super) fn generate_server_reducers_file(
    module: &ModuleDef,
    options: &CodegenOptions,
    root_module: &str,
) -> OutputFile {
    let reducer_data: Vec<ReducerData> = iter_reducers(module, options.visibility)
        .filter(|r| is_reducer_invokable(r))
        .map(|r| ReducerData {
            name_camel: rescript_field_name(r.accessor_name.deref().to_case(Case::Camel)),
            // Dotted path — the template does `open {root_module}` so `Reducers.Foo` resolves.
            module: format!("Reducers.{}", r.name.deref().to_case(Case::Pascal)),
            has_args: !r.params_for_generate.elements.is_empty(),
        })
        .collect();

    let has_reducers = !reducer_data.is_empty();

    let reducer_wrappers: Vec<ServerReducerWrapperRes> = reducer_data
        .iter()
        .map(|r| ServerReducerWrapperRes {
            name_camel: &r.name_camel,
            module: &r.module,
            has_args: r.has_args,
        })
        .collect();

    let reducer_type_fields: Vec<ServerReducerTypeFieldRes> = reducer_data
        .iter()
        .map(|r| ServerReducerTypeFieldRes {
            name_camel: &r.name_camel,
            module: &r.module,
            has_args: r.has_args,
        })
        .collect();

    let reducer_value_fields: Vec<ServerReducerValueFieldRes> = reducer_data
        .iter()
        .map(|r| ServerReducerValueFieldRes {
            name_camel: &r.name_camel,
        })
        .collect();

    OutputFile {
        filename: format!("{root_module}__ServerReducers.res"),
        code: StdbServerReducersRes {
            header: AutoGenHeaderRes,
            reducer_wrappers,
            reducer_type_fields,
            reducer_value_fields,
            has_reducers,
            root_module,
        }
        .to_string(),
    }
}
