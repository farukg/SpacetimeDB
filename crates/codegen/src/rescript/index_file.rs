//! `index.res` generator — module re-exports for tables, reducers, and procedures.

use super::helpers::{procedure_module_name, reducer_module_name, table_module_name};
use super::templates::{AutoGenHeaderRes, IndexRes, ModuleAliasRes};
use crate::util::{is_reducer_invokable, iter_procedures, iter_reducers, iter_table_names_and_types};
use crate::{CodegenOptions, OutputFile};

use convert_case::{Case, Casing};
use spacetimedb_schema::def::ModuleDef;
use std::ops::Deref;

/// Intermediate owned data for a module alias.
struct AliasData {
    alias: String,
    target: String,
}

pub(super) fn generate_index_file(module: &ModuleDef, options: &CodegenOptions, root_module: &str) -> OutputFile {
    let table_data: Vec<AliasData> = iter_table_names_and_types(module, options.visibility)
        .map(|(_, accessor_name, _)| AliasData {
            alias: accessor_name.deref().to_case(Case::Pascal),
            target: table_module_name(root_module, accessor_name),
        })
        .collect();

    let reducer_data: Vec<AliasData> = iter_reducers(module, options.visibility)
        .filter(|r| is_reducer_invokable(r))
        .map(|r| AliasData {
            alias: r.accessor_name.deref().to_case(Case::Pascal),
            target: reducer_module_name(root_module, &r.name),
        })
        .collect();

    let procedure_data: Vec<AliasData> = iter_procedures(module, options.visibility)
        .map(|p| AliasData {
            alias: p.accessor_name.deref().to_case(Case::Pascal),
            target: procedure_module_name(root_module, &p.accessor_name),
        })
        .collect();

    let table_aliases: Vec<ModuleAliasRes> = table_data
        .iter()
        .map(|d| ModuleAliasRes {
            alias: &d.alias,
            target: &d.target,
        })
        .collect();

    let reducer_aliases: Vec<ModuleAliasRes> = reducer_data
        .iter()
        .map(|d| ModuleAliasRes {
            alias: &d.alias,
            target: &d.target,
        })
        .collect();

    let procedure_aliases: Vec<ModuleAliasRes> = procedure_data
        .iter()
        .map(|d| ModuleAliasRes {
            alias: &d.alias,
            target: &d.target,
        })
        .collect();

    OutputFile {
        filename: "index.res".to_string(),
        code: IndexRes {
            header: AutoGenHeaderRes,
            table_aliases,
            reducer_aliases,
            procedure_aliases,
        }
        .to_string(),
    }
}
