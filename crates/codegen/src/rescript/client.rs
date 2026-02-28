//! StdbClient.res generation — db record aggregating all tables + connection accessors.

use super::helpers::{rescript_field_name, table_module_name};
use super::templates::{AutoGenHeaderRes, DbFieldRes, StdbClientRes};
use crate::util::{iter_tables, iter_views};
use crate::{CodegenOptions, OutputFile};

use convert_case::{Case, Casing};
use spacetimedb_schema::def::ModuleDef;
use std::ops::Deref;

/// Intermediate owned data for a db field (strings live here, templates borrow from here).
struct DbFieldData {
    accessor: String,
    camel: String,
    table_module: String,
}

/// Generates `StdbClient.res`.
pub(super) fn generate_client_file(module: &ModuleDef, options: &CodegenOptions) -> OutputFile {
    let mut fields: Vec<DbFieldData> = Vec::new();

    for table in iter_tables(module, options.visibility) {
        let accessor = table.accessor_name.deref().to_string();
        let camel = rescript_field_name(accessor.to_case(Case::Camel));
        let table_module = table_module_name(&table.accessor_name);
        fields.push(DbFieldData {
            accessor,
            camel,
            table_module,
        });
    }

    for view in iter_views(module) {
        let accessor = view.accessor_name.deref().to_string();
        let camel = rescript_field_name(accessor.to_case(Case::Camel));
        let view_module = table_module_name(&view.accessor_name);
        fields.push(DbFieldData {
            accessor,
            camel,
            table_module: view_module,
        });
    }

    let db_fields: Vec<DbFieldRes> = fields
        .iter()
        .map(|f| DbFieldRes {
            accessor: &f.accessor,
            camel: &f.camel,
            table_module: &f.table_module,
        })
        .collect();

    OutputFile {
        filename: "StdbClient.res".to_string(),
        code: StdbClientRes {
            header: AutoGenHeaderRes,
            db_fields,
        }
        .to_string(),
    }
}
