//! Namespace gateway generators — root, tables, reducers, procedures.
//!
//! Emits:
//! - `{root}.res` — root gateway with aliases to Types, Schema, Client, etc.
//! - `{root}__Tables.res` — table namespace gateway
//! - `{root}__Reducers.res` — reducer namespace gateway
//! - `{root}__Procedures.res` — procedure namespace gateway (if any procedures exist)

use super::helpers::{procedure_module_name, reducer_module_name, table_module_name};
use super::templates::{AutoGenHeaderRes, ModuleAliasRes, NamespaceGatewayRes};
use crate::util::{is_reducer_invokable, iter_procedures, iter_reducers, iter_table_names_and_types};
use crate::{AsyncStyle, CodegenOptions, OutputFile};

use convert_case::{Case, Casing};
use spacetimedb_schema::def::ModuleDef;
use std::ops::Deref;

/// Intermediate owned data for a module alias.
struct AliasData {
    alias: String,
    target: String,
}

/// Emit all namespace gateway files.
pub(super) fn generate_gateway_files(
    module: &ModuleDef,
    options: &CodegenOptions,
    root_module: &str,
    async_style: AsyncStyle,
) -> Vec<OutputFile> {
    let mut files = Vec::new();

    // ── Table gateway: {root}__Tables.res ─────────────────────────────────
    let table_data: Vec<AliasData> = iter_table_names_and_types(module, options.visibility)
        .map(|(_, accessor_name, _)| AliasData {
            alias: accessor_name.deref().to_case(Case::Pascal),
            target: table_module_name(root_module, accessor_name),
        })
        .collect();

    let table_aliases: Vec<ModuleAliasRes> = table_data
        .iter()
        .map(|d| ModuleAliasRes {
            alias: &d.alias,
            target: &d.target,
        })
        .collect();

    files.push(OutputFile {
        filename: format!("{root_module}__Tables.res"),
        code: NamespaceGatewayRes {
            header: AutoGenHeaderRes,
            aliases: table_aliases,
        }
        .to_string(),
    });

    // ── Reducer gateway: {root}__Reducers.res ─────────────────────────────
    let reducer_data: Vec<AliasData> = iter_reducers(module, options.visibility)
        .filter(|r| is_reducer_invokable(r))
        .map(|r| AliasData {
            alias: r.accessor_name.deref().to_case(Case::Pascal),
            target: reducer_module_name(root_module, &r.name),
        })
        .collect();

    let reducer_aliases: Vec<ModuleAliasRes> = reducer_data
        .iter()
        .map(|d| ModuleAliasRes {
            alias: &d.alias,
            target: &d.target,
        })
        .collect();

    files.push(OutputFile {
        filename: format!("{root_module}__Reducers.res"),
        code: NamespaceGatewayRes {
            header: AutoGenHeaderRes,
            aliases: reducer_aliases,
        }
        .to_string(),
    });

    // ── Procedure gateway: {root}__Procedures.res (only if procedures exist)
    let procedure_data: Vec<AliasData> = iter_procedures(module, options.visibility)
        .map(|p| AliasData {
            alias: p.accessor_name.deref().to_case(Case::Pascal),
            target: procedure_module_name(root_module, &p.accessor_name),
        })
        .collect();

    if !procedure_data.is_empty() {
        let procedure_aliases: Vec<ModuleAliasRes> = procedure_data
            .iter()
            .map(|d| ModuleAliasRes {
                alias: &d.alias,
                target: &d.target,
            })
            .collect();

        files.push(OutputFile {
            filename: format!("{root_module}__Procedures.res"),
            code: NamespaceGatewayRes {
                header: AutoGenHeaderRes,
                aliases: procedure_aliases,
            }
            .to_string(),
        });
    }

    // ── Root gateway: {root}.res ──────────────────────────────────────────
    let mut root_aliases_data: Vec<AliasData> = vec![
        AliasData {
            alias: "Types".to_string(),
            target: format!("{root_module}__Types"),
        },
        AliasData {
            alias: "Schema".to_string(),
            target: format!("{root_module}__Schema"),
        },
        AliasData {
            alias: "Client".to_string(),
            target: format!("{root_module}__Client"),
        },
        AliasData {
            alias: "Sdk".to_string(),
            target: format!("{root_module}__Sdk"),
        },
        AliasData {
            alias: "Labels".to_string(),
            target: format!("{root_module}__Labels"),
        },
        AliasData {
            alias: "Display".to_string(),
            target: format!("{root_module}__Display"),
        },
        AliasData {
            alias: "Tables".to_string(),
            target: format!("{root_module}__Tables"),
        },
        AliasData {
            alias: "Reducers".to_string(),
            target: format!("{root_module}__Reducers"),
        },
        AliasData {
            alias: "ServerReducers".to_string(),
            target: format!("{root_module}__ServerReducers"),
        },
        AliasData {
            alias: "React".to_string(),
            target: format!("{root_module}__React"),
        },
        AliasData {
            alias: "Provider".to_string(),
            target: format!("{root_module}__Provider"),
        },
    ];

    if !procedure_data.is_empty() {
        root_aliases_data.push(AliasData {
            alias: "Procedures".to_string(),
            target: format!("{root_module}__Procedures"),
        });
    }

    if async_style != AsyncStyle::Promise {
        root_aliases_data.push(AliasData {
            alias: "Async".to_string(),
            target: format!("{root_module}__Async"),
        });
    }

    let root_aliases: Vec<ModuleAliasRes> = root_aliases_data
        .iter()
        .map(|d| ModuleAliasRes {
            alias: &d.alias,
            target: &d.target,
        })
        .collect();

    files.push(OutputFile {
        filename: format!("{root_module}.res"),
        code: NamespaceGatewayRes {
            header: AutoGenHeaderRes,
            aliases: root_aliases,
        }
        .to_string(),
    });

    files
}
