//! StdbClient.res generation — core SDK opaque types + db record aggregating all tables.
//!
//! This is the single import point for connection, db, reducers, and eventCtx.
//! Each per-table file references `StdbTypes.eventCtx` and `StdbTypes.reducers`.
//! Each per-reducer file references `StdbTypes.connection` and `StdbTypes.reducers`.

use super::helpers::{rescript_field_name, table_module_name};
use crate::code_indenter::CodeIndenter;

use crate::util::{iter_tables, iter_views, print_auto_generated_file_comment};
use crate::{CodegenOptions, OutputFile};

use convert_case::{Case, Casing};
use spacetimedb_schema::def::ModuleDef;
use std::ops::Deref;

/// Generates `StdbClient.res` — core SDK opaque types + db record aggregating all tables.
///
/// Contains:
/// - Opaque SDK type placeholders
/// - `type db = { ... }` record with @as-annotated table handle fields
/// - `@get external db` and `@get external reducers` accessors
pub(super) fn generate_client_file(module: &ModuleDef, options: &CodegenOptions) -> OutputFile {
    let mut output = CodeIndenter::new(String::new(), super::INDENT);
    let out = &mut output;

    print_auto_generated_file_comment(out);
    writeln!(out, "");

    // ── Opaque SDK types ──
    writeln!(
        out,
        "// Opaque SDK types — hold JS class instances from the SpacetimeDB SDK"
    );
    writeln!(out, "");
    writeln!(out, "");
    writeln!(out, "");
    writeln!(out, "");

    // ── DB record type (SIG/ADR-017: @as for compile-time-safe table access) ──
    // Each field maps camelCase ReScript name → snake_case runtime key.
    // The handle type for each table is defined in its own Stdb*Table.res module.
    let tables: Vec<_> = iter_tables(module, options.visibility).collect();

    writeln!(
        out,
        "// DB record — @as maps camelCase fields to snake_case runtime keys"
    );
    writeln!(out, "type db = {{");
    out.indent(1);
    for table in &tables {
        let accessor = table.accessor_name.deref();
        let camel = rescript_field_name(accessor.to_case(Case::Camel));
        let table_module = table_module_name(&table.accessor_name);
        // @as string = raw accessor_name (snake_case) — SSOT from REMOTE_MODULE
        writeln!(out, "@as(\"{accessor}\") {camel}: {table_module}.handle,");
    }
    // Views use the same module naming as tables (generate_view_file converts ViewDef → TableDef)
    for view in iter_views(module) {
        let accessor = view.accessor_name.deref();
        let camel = rescript_field_name(accessor.to_case(Case::Camel));
        let view_module = table_module_name(&view.accessor_name);
        writeln!(out, "@as(\"{accessor}\") {camel}: {view_module}.handle,");
    }
    out.dedent(1);
    writeln!(out, "}}");
    writeln!(out, "");

    // ── DB + reducers access chain ──
    writeln!(out, "// DB and reducers access from connection");
    writeln!(out, "@get external db: StdbTypes.connection => db = \"db\"");
    writeln!(
        out,
        "@get external reducers: StdbTypes.connection => StdbTypes.reducers = \"reducers\""
    );

    OutputFile {
        filename: "StdbClient.res".to_string(),
        code: output.into_inner(),
    }
}
