//! Server-side reducer wrappers codegen — generates `StdbServerReducers.res`.

use super::helpers::{rescript_field_name, sibling_opens};
use super::templates::{AutoGenHeaderRes, ServerReducerWrapperRes, StdbServerReducersRes};
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

    let reducer_type_fields = reducer_data
        .iter()
        .map(|r| {
            if r.has_args {
                format!(
                    "    {}: {}.args => Fx.call<result<unit, Fx.error>>,",
                    r.name_camel, r.module
                )
            } else {
                format!("    {}: unit => Fx.call<result<unit, Fx.error>>,", r.name_camel)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let reducer_value_fields = reducer_data
        .iter()
        .map(|r| format!("    {}: {},", r.name_camel, r.name_camel))
        .collect::<Vec<_>>()
        .join("\n");

    let opens = sibling_opens(root_module, &["Sdk", "Reducers", "Fx"]);
    OutputFile {
        filename: format!("{root_module}__ServerReducers.res"),
        code: StdbServerReducersRes {
            header: AutoGenHeaderRes,
            reducer_wrappers,
            reducer_type_fields: &reducer_type_fields,
            reducer_value_fields: &reducer_value_fields,
            has_reducers,
            sibling_opens: &opens,
        }
        .to_string(),
    }
}
