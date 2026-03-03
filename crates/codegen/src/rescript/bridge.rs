//! `{root}__Bridge.res` — per-schema table configs for observer hooks.
//!
//! Emitted when `async_style ∈ {Observer, All}`. Contains one `let` binding
//! per table, each producing a `Hooks.tableConfig<Table.t>` via `Hooks.mkTable`.
//!
//! Consumer usage:
//! ```rescript
//! let rows = Hooks.useRows(Bridge.myReceipts)
//! ```

use super::helpers::{rescript_field_name, table_module_name};
use super::templates::{AutoGenHeaderRes, BridgeTableEntryRes, StdbBridgeRes};
use crate::util::iter_table_names_and_types;
use crate::{CodegenOptions, OutputFile};

use convert_case::{Case, Casing};
use spacetimedb_schema::def::ModuleDef;
use std::ops::Deref;

pub(super) fn generate_bridge_file(module: &ModuleDef, options: &CodegenOptions, root_module: &str) -> OutputFile {
    // Collect owned data first, then build borrowed template entries.
    struct TableData {
        config_name: String,
        accessor: String,
        table_module: String,
    }

    let table_data: Vec<TableData> = iter_table_names_and_types(module, options.visibility)
        .map(|(_, accessor_name, _)| {
            let camel = rescript_field_name(accessor_name.deref().to_case(Case::Camel));
            TableData {
                config_name: camel.clone(),
                accessor: camel,
                table_module: table_module_name(root_module, accessor_name),
            }
        })
        .collect();

    let table_entries: Vec<BridgeTableEntryRes> = table_data
        .iter()
        .map(|d| BridgeTableEntryRes {
            config_name: &d.config_name,
            accessor: &d.accessor,
            table_module: &d.table_module,
        })
        .collect();

    OutputFile {
        filename: format!("{root_module}__Bridge.res"),
        code: StdbBridgeRes {
            header: AutoGenHeaderRes,
            root_module,
            table_entries,
        }
        .to_string(),
    }
}
