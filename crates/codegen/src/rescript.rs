use crate::util::{
    collect_case, is_reducer_invokable, iter_constraints, iter_indexes, iter_procedures, iter_reducers,
    iter_table_names_and_types, iter_tables, iter_types, iter_views, print_auto_generated_file_comment,
};
use crate::{CodegenOptions, OutputFile};

use super::code_indenter::{CodeIndenter, Indenter};
use super::util::type_ref_name;
use super::Lang;

use convert_case::{Case, Casing};
use spacetimedb_lib::sats::layout::PrimitiveType;
use spacetimedb_lib::version::spacetimedb_lib_version;
use spacetimedb_primitives::ColId;
use spacetimedb_schema::def::{ConstraintDef, IndexDef, ModuleDef, ProcedureDef, ReducerDef, TableDef, TypeDef};
use spacetimedb_schema::identifier::Identifier;
use spacetimedb_schema::reducer_name::ReducerName;
use spacetimedb_schema::schema::TableSchema;
use spacetimedb_schema::type_for_generate::{AlgebraicTypeDef, AlgebraicTypeUse};
use std::fmt::{self, Write};
use std::iter;
use std::ops::Deref;

const INDENT: &str = "  ";

pub struct ReScript;

impl Lang for ReScript {
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

        write_record_type(module, out, "t", &product_def.elements);
        writeln!(out, "let tableName = \"{}\"", table.name);

        OutputFile {
            filename: format!("{}.res", table_module_name(&table.accessor_name)),
            code: output.into_inner(),
        }
    }

    fn generate_type_files(&self, _module: &ModuleDef, _typ: &TypeDef) -> Vec<OutputFile> {
        vec![]
    }

    fn generate_reducer_file(&self, _module: &ModuleDef, _reducer: &ReducerDef) -> OutputFile {
        OutputFile {
            filename: "StdbDummyReducer.res".to_string(),
            code: String::new(),
        }
    }

    fn generate_procedure_file(&self, module: &ModuleDef, procedure: &ProcedureDef) -> OutputFile {
        let mut output = CodeIndenter::new(String::new(), INDENT);
        let out = &mut output;

        print_auto_generated_file_comment(out);
        writeln!(out, "");

        write_record_type(module, out, "params", &procedure.params_for_generate.elements);
        writeln!(out, "");
        write!(out, "type result = ");
        write_res_type(module, out, &procedure.return_type_for_generate);
        writeln!(out, "");
        writeln!(out, "let procedureName = \"{}\"", procedure.name);

        OutputFile {
            filename: format!("{}.res", procedure_module_name(&procedure.accessor_name)),
            code: output.into_inner(),
        }
    }

    fn generate_global_files(&self, module: &ModuleDef, options: &CodegenOptions) -> Vec<OutputFile> {
        vec![
            generate_types_file(module),
            generate_reducers_file(module, options),
            generate_index_file(module, options),
            generate_schema_file(module, options),
            generate_bindings_file(module, options),
        ]
    }
}

fn generate_types_file(module: &ModuleDef) -> OutputFile {
    let mut output = CodeIndenter::new(String::new(), INDENT);
    let out = &mut output;

    print_auto_generated_file_comment(out);
    writeln!(out, "");

    // ── Opaque timestamp type ──
    // SDK delivers Timestamp class instances. We bind them as opaque types
    // and expose methods via @send externals — zero normalization needed.
    writeln!(out, "// Opaque SDK Timestamp — use toDate, toMillis, or toFloatMs");
    writeln!(out, "type timestamp");
    writeln!(out, "@send external toMillis: (timestamp) => bigint = \"toMillis\"");
    writeln!(out, "@send external toDate: (timestamp) => Date.t = \"toDate\"");
    writeln!(
        out,
        "let toFloatMs = (ts: timestamp): float => ts->toMillis->BigInt.toFloat"
    );
    writeln!(out, "");

    // Collect all types first so we know whether to emit `type rec` or `and`.
    // ReScript requires `type rec ... and ...` when types reference each other
    // (including forward references, which are common when product types come
    // before the enum types they reference).
    let types: Vec<_> = iter_types(module).collect();
    if types.is_empty() {
        return OutputFile {
            filename: "StdbTypes.res".to_string(),
            code: output.into_inner(),
        };
    }

    for (i, ty) in types.iter().enumerate() {
        let type_name = rescript_type_name(collect_case(Case::Pascal, ty.accessor_name.name_segments()));
        let keyword = if i == 0 { "type rec" } else { "and" };
        match &module.typespace_for_generate()[ty.ty] {
            AlgebraicTypeDef::Product(product) => {
                write_record_type_rec(module, out, keyword, &type_name, &product.elements)
            }
            AlgebraicTypeDef::Sum(sum) => write_sum_type_rec(module, out, keyword, &type_name, &sum.variants),
            AlgebraicTypeDef::PlainEnum(plain_enum) => {
                // Emit @tag("tag") so ReScript generates {tag: "X", value: ...} objects
                // matching the SpacetimeDB SDK's tagged enum representation.
                // Unit variants get {value: unit} payload so the tag field is always present
                // (bare payloadless variants would compile to plain strings).
                writeln!(out, "@tag(\"tag\")");
                writeln!(out, "{keyword} {type_name} =");
                out.indent(1);
                for variant in &plain_enum.variants {
                    let constructor = rescript_constructor_name(variant.deref());
                    writeln!(out, "| {constructor}({{value: unit}})");
                }
                out.dedent(1);
                writeln!(out, "");
            }
        }
    }

    OutputFile {
        filename: "StdbTypes.res".to_string(),
        code: output.into_inner(),
    }
}

fn generate_index_file(module: &ModuleDef, options: &CodegenOptions) -> OutputFile {
    let mut output = CodeIndenter::new(String::new(), INDENT);
    let out = &mut output;

    print_auto_generated_file_comment(out);
    writeln!(out, "");
    writeln!(out, "module StdbTypes = StdbTypes");
    writeln!(out, "");

    writeln!(out, "module Tables = {{");
    out.indent(1);
    for (_, accessor_name, _) in iter_table_names_and_types(module, options.visibility) {
        let alias = accessor_name.deref().to_case(Case::Pascal);
        let table_module = table_module_name(accessor_name);
        writeln!(out, "module {alias} = {table_module}");
    }
    out.dedent(1);
    writeln!(out, "}}");
    writeln!(out, "");

    writeln!(out, "module Reducers = StdbReducers");
    writeln!(out, "");

    writeln!(out, "module Procedures = {{");
    out.indent(1);
    for procedure in iter_procedures(module, options.visibility) {
        let alias = procedure.accessor_name.deref().to_case(Case::Pascal);
        let procedure_module = procedure_module_name(&procedure.accessor_name);
        writeln!(out, "module {alias} = {procedure_module}");
    }
    out.dedent(1);
    writeln!(out, "}}");

    OutputFile {
        filename: "index.res".to_string(),
        code: output.into_inner(),
    }
}

fn generate_reducers_file(module: &ModuleDef, options: &CodegenOptions) -> OutputFile {
    let mut output = CodeIndenter::new(String::new(), INDENT);
    let out = &mut output;

    print_auto_generated_file_comment(out);
    writeln!(out, "");

    writeln!(
        out,
        "let encodeOption = (opt: option<'a>): Null.t<{{\"some\": 'a}}> => {{"
    );
    writeln!(out, "  switch opt {{");
    writeln!(out, "  | None => Null.null");
    writeln!(out, "  | Some(v) => Null.make({{\"some\": v}})");
    writeln!(out, "  }}");
    writeln!(out, "}}");
    writeln!(out, "");

    for reducer in iter_reducers(module, options.visibility) {
        let reducer_name = reducer.name.clone();
        let accessor_name = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
        let external_name = format!("callReducerArgs_{}", accessor_name);

        // write external
        writeln!(out, "@module(\"../api/stdb-server.mjs\")");
        writeln!(out, "external {external_name}: (");
        out.indent(1);

        let elements = &reducer.params_for_generate.elements;
        if elements.is_empty() {
            writeln!(out, "@as(\"{reducer_name}\") _,");
            writeln!(out, "unit");
        } else {
            writeln!(out, "@as(\"{reducer_name}\") _,");
            for (i, (_field, ty)) in elements.iter().enumerate() {
                write_res_encoded_type(module, out, ty, false);
                if i < elements.len() - 1 {
                    writeln!(out, ",");
                } else {
                    writeln!(out, "");
                }
            }
        }

        out.dedent(1);
        writeln!(out, ") => promise<unit> = \"callReducerArgs\"");
        writeln!(out, "");

        // write wrapper
        write!(out, "let {} = (", accessor_name);
        if elements.is_empty() {
            write!(out, ")");
        } else {
            writeln!(out, "");
            out.indent(1);
            for (i, (field, ty)) in elements.iter().enumerate() {
                let field_name = rescript_field_name(field.deref().to_case(Case::Camel));
                write!(out, "~{}: ", field_name);
                write_res_type_ctx(module, out, ty, false);
                if i < elements.len() - 1 {
                    writeln!(out, ",");
                } else {
                    writeln!(out, "");
                }
            }
            out.dedent(1);
            write!(out, ")");
        }

        if elements.is_empty() {
            writeln!(out, " => {}()", external_name);
            writeln!(out, "");
            continue;
        }
        writeln!(out, " => {}(", external_name);
        out.indent(1);

        for (i, (field, ty)) in elements.iter().enumerate() {
            let field_name = rescript_field_name(field.deref().to_case(Case::Camel));
            let mut mapped_name = field_name.clone();
            if let AlgebraicTypeUse::Option(_) = ty {
                mapped_name = format!("encodeOption({})", field_name);
            }
            write!(out, "{}", mapped_name);
            if i < elements.len() - 1 {
                writeln!(out, ",");
            } else {
                writeln!(out, "");
            }
        }
        out.dedent(1);
        if !elements.is_empty() {
            writeln!(out, ")");
        } else {
            writeln!(out, "");
        }
        writeln!(out, "");
    }

    OutputFile {
        filename: "StdbReducers.res".into(),
        code: output.into_inner(),
    }
}

/// Used for per-table / per-reducer files (outside StdbTypes.res): emits `type <name> = { ... }`.
fn write_record_type(module: &ModuleDef, out: &mut Indenter, name: &str, elements: &[(Identifier, AlgebraicTypeUse)]) {
    write_record_type_ctx(module, out, name, elements, false);
}

fn write_record_type_ctx(
    module: &ModuleDef,
    out: &mut Indenter,
    name: &str,
    elements: &[(Identifier, AlgebraicTypeUse)],
    in_types_file: bool,
) {
    write_record_type_kw(module, out, "type", name, elements, in_types_file);
}

/// Used inside StdbTypes.res: emits `<keyword> <name> = { ... }` where keyword is `type rec` or `and`.
fn write_record_type_rec(
    module: &ModuleDef,
    out: &mut Indenter,
    keyword: &str,
    name: &str,
    elements: &[(Identifier, AlgebraicTypeUse)],
) {
    write_record_type_kw(module, out, keyword, name, elements, true);
}

fn write_record_type_kw(
    module: &ModuleDef,
    out: &mut Indenter,
    keyword: &str,
    name: &str,
    elements: &[(Identifier, AlgebraicTypeUse)],
    in_types_file: bool,
) {
    if elements.is_empty() {
        writeln!(out, "{keyword} {name} = unit");
        writeln!(out, "");
        return;
    }

    writeln!(out, "{keyword} {name} = {{");
    out.indent(1);
    for (field, ty) in elements {
        let field_name = rescript_field_name(field.deref().to_case(Case::Camel));
        write!(out, "{field_name}: ");
        write_res_type_ctx(module, out, ty, in_types_file);
        writeln!(out, ",");
    }
    out.dedent(1);
    writeln!(out, "}}");
    writeln!(out, "");
}

fn write_sum_type_rec(
    module: &ModuleDef,
    out: &mut Indenter,
    keyword: &str,
    name: &str,
    variants: &[(Identifier, AlgebraicTypeUse)],
) {
    write_sum_type_ctx(module, out, keyword, name, variants, true);
}

fn write_sum_type_ctx(
    module: &ModuleDef,
    out: &mut Indenter,
    keyword: &str,
    name: &str,
    variants: &[(Identifier, AlgebraicTypeUse)],
    in_types_file: bool,
) {
    // Sum types (enums with potential payloads) also need @tag("tag") for SDK interop.
    // Unit variants get {value: unit} payload to match SDK's {tag: "X", value: {}} objects.
    writeln!(out, "@tag(\"tag\")");
    writeln!(out, "{keyword} {name} =");
    out.indent(1);
    for (variant_name, variant_type) in variants {
        let constructor = rescript_constructor_name(variant_name.deref());
        if matches!(variant_type, AlgebraicTypeUse::Unit) {
            writeln!(out, "| {constructor}({{value: unit}})");
        } else {
            write!(out, "| {constructor}(");
            write_res_type_ctx(module, out, variant_type, in_types_file);
            writeln!(out, ")");
        }
    }
    out.dedent(1);
    writeln!(out, "");
}

fn write_res_encoded_type(module: &ModuleDef, out: &mut Indenter, ty: &AlgebraicTypeUse, in_types_file: bool) {
    match ty {
        AlgebraicTypeUse::Option(inner) => {
            write!(out, "Null.t<{{\"some\": ");
            write_res_encoded_type(module, out, inner, in_types_file);
            write!(out, "}}>");
        }
        AlgebraicTypeUse::Array(inner) => {
            write!(out, "array<");
            write_res_encoded_type(module, out, inner, in_types_file);
            write!(out, ">");
        }
        _ => write_res_type_ctx(module, out, ty, in_types_file),
    }
}

fn write_res_type(module: &ModuleDef, out: &mut Indenter, ty: &AlgebraicTypeUse) {
    write_res_type_ctx(module, out, ty, false);
}

fn write_res_type_ctx(module: &ModuleDef, out: &mut Indenter, ty: &AlgebraicTypeUse, in_types_file: bool) {
    match ty {
        AlgebraicTypeUse::Unit => {
            write!(out, "unit");
        }
        AlgebraicTypeUse::Never => {
            write!(out, "unit");
        }
        AlgebraicTypeUse::Identity | AlgebraicTypeUse::ConnectionId | AlgebraicTypeUse::Uuid => {
            write!(out, "string");
        }
        AlgebraicTypeUse::Timestamp | AlgebraicTypeUse::TimeDuration => {
            if in_types_file {
                write!(out, "timestamp");
            } else {
                write!(out, "StdbTypes.timestamp");
            }
        }
        AlgebraicTypeUse::ScheduleAt => {
            write!(out, "[ #Interval(float) | #Time(float) ]");
        }
        AlgebraicTypeUse::Option(inner) => {
            write!(out, "option<");
            write_res_type_ctx(module, out, inner, in_types_file);
            write!(out, ">");
        }
        AlgebraicTypeUse::Result { ok_ty, err_ty } => {
            write!(out, "result<");
            write_res_type_ctx(module, out, ok_ty, in_types_file);
            write!(out, ", ");
            write_res_type_ctx(module, out, err_ty, in_types_file);
            write!(out, ">");
        }
        AlgebraicTypeUse::Primitive(prim) => match prim {
            PrimitiveType::Bool => {
                write!(out, "bool");
            }
            PrimitiveType::I8
            | PrimitiveType::U8
            | PrimitiveType::I16
            | PrimitiveType::U16
            | PrimitiveType::I32
            | PrimitiveType::U32 => {
                write!(out, "int");
            }
            PrimitiveType::I64
            | PrimitiveType::U64
            | PrimitiveType::I128
            | PrimitiveType::U128
            | PrimitiveType::I256
            | PrimitiveType::U256 => {
                write!(out, "bigint");
            }
            PrimitiveType::F32 | PrimitiveType::F64 => {
                write!(out, "float");
            }
        },
        AlgebraicTypeUse::String => {
            write!(out, "string");
        }
        AlgebraicTypeUse::Array(inner) => {
            write!(out, "array<");
            write_res_type_ctx(module, out, inner, in_types_file);
            write!(out, ">");
        }
        AlgebraicTypeUse::Ref(reference) => {
            let reference_name = rescript_type_name(type_ref_name(module, *reference));
            if in_types_file {
                write!(out, "{reference_name}");
            } else {
                write!(out, "StdbTypes.{reference_name}");
            }
        }
    }
}

fn table_module_name(table_name: &Identifier) -> String {
    format!("Stdb{}Table", table_name.deref().to_case(Case::Pascal))
}

fn reducer_module_name(reducer_name: &ReducerName) -> String {
    format!("Stdb{}Reducer", reducer_name.deref().to_case(Case::Pascal))
}

fn procedure_module_name(procedure_name: &Identifier) -> String {
    format!("Stdb{}Procedure", procedure_name.deref().to_case(Case::Pascal))
}

fn rescript_type_name(type_name_pascal: String) -> String {
    rescript_field_name(type_name_pascal.to_case(Case::Camel))
}

fn rescript_field_name(name: String) -> String {
    match name.as_str() {
        "and" | "assert" | "constraint" | "exception" | "external" | "for" | "if" | "in" | "include" | "let"
        | "module" | "mutable" | "open" | "private" | "rec" | "switch" | "type" | "when" | "while" => {
            format!("{name}_")
        }
        _ => name,
    }
}

fn rescript_constructor_name(name: &str) -> String {
    // If the name is already PascalCase (no underscores, starts with uppercase),
    // use as-is to preserve acronym casing (e.g. UG, AG, OHG, KG, GmbH).
    // `convert_case::Case::Pascal` would mangle these: UG → Ug, AG → Ag.
    let first_char = name.chars().next();
    if first_char.is_some_and(|c| c.is_ascii_uppercase()) && !name.contains('_') {
        return name.to_string();
    }

    let pascal = name.to_case(Case::Pascal);
    if pascal.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("V{pascal}")
    } else {
        pascal
    }
}

// ---------------------------------------------------------------------------
// StdbSchema.mjs — pure JS runtime schema for the SpacetimeDB SDK.
//
// This file contains ONLY the REMOTE_MODULE object needed by the SDK for
// BSATN deserialization. No TypeScript, no types, no classes — just the
// runtime data that feeds `DbConnectionImpl` and `DbConnectionBuilder`.
//
// ReScript types live in StdbTypes.res (already generated). This file is the
// bridge between the ReScript world and the SDK's internal type system.
// ---------------------------------------------------------------------------

fn generate_schema_file(module: &ModuleDef, options: &CodegenOptions) -> OutputFile {
    let mut output = CodeIndenter::new(String::new(), INDENT);
    let out = &mut output;

    writeln!(
        out,
        "// THIS FILE IS AUTOMATICALLY GENERATED BY SPACETIMEDB (ReScript codegen)."
    );
    writeln!(
        out,
        "// EDITS WILL NOT BE SAVED. MODIFY TABLES IN YOUR SERVER MODULE INSTEAD."
    );
    writeln!(out, "//");
    writeln!(out, "// Pure JS runtime schema — no TypeScript, no ReScript.");
    writeln!(out, "// Consumed by client.mjs and stdb-server.mjs as REMOTE_MODULE.");
    writeln!(out, "");
    writeln!(
        out,
        "import {{ schema, table, reducers, reducerSchema, procedures, procedureSchema, t }} from \"spacetimedb\";"
    );
    writeln!(out, "");

    // --- Type builders (enums + objects) ---
    // These mirror types.ts but as plain JS const declarations.
    // Forward-reference handling: use `get` getters for Ref fields.

    let types: Vec<_> = iter_types(module).collect();
    if !types.is_empty() {
        writeln!(out, "// Type builders");
    }
    for ty in &types {
        let type_name = collect_case(Case::Pascal, ty.accessor_name.name_segments());
        match &module.typespace_for_generate()[ty.ty] {
            AlgebraicTypeDef::Product(product) => {
                write_schema_object_builder(module, out, &type_name, &product.elements);
            }
            AlgebraicTypeDef::Sum(sum) => {
                write_schema_enum_builder(module, out, &type_name, &sum.variants);
            }
            AlgebraicTypeDef::PlainEnum(plain_enum) => {
                let unit_variants: Vec<(Identifier, AlgebraicTypeUse)> = plain_enum
                    .variants
                    .iter()
                    .cloned()
                    .map(|var| (var, AlgebraicTypeUse::Unit))
                    .collect();
                write_schema_enum_builder(module, out, &type_name, &unit_variants);
            }
        }
    }
    if !types.is_empty() {
        writeln!(out, "");
    }

    // --- Table row builders ---
    writeln!(out, "// Table row builders");
    for table in iter_tables(module, options.visibility) {
        let type_ref = table.product_type_ref;
        let product_def = module.typespace_for_generate()[type_ref].as_product().unwrap();
        let row_name = format!("{}Row", table.accessor_name.deref().to_case(Case::Pascal));

        writeln!(out, "const {} = t.row({{", row_name);
        out.indent(1);
        write_schema_object_fields(module, out, &product_def.elements, table.primary_key, true, true);
        out.dedent(1);
        writeln!(out, "}});");
    }
    for view in iter_views(module) {
        let type_ref = view.product_type_ref;
        let product_def = module.typespace_for_generate()[type_ref].as_product().unwrap();
        let row_name = format!("{}Row", view.accessor_name.deref().to_case(Case::Pascal));

        writeln!(out, "const {} = t.row({{", row_name);
        out.indent(1);
        write_schema_object_fields(module, out, &product_def.elements, None, true, true);
        out.dedent(1);
        writeln!(out, "}});");
    }
    writeln!(out, "");

    // --- Reducer arg builders ---
    writeln!(out, "// Reducer arg builders");
    for reducer in iter_reducers(module, options.visibility) {
        if !is_reducer_invokable(reducer) {
            continue;
        }
        let args_name = format!("{}Args", reducer.accessor_name.deref().to_case(Case::Pascal));
        let elements = &reducer.params_for_generate.elements;

        write!(out, "const {} = {{", args_name);
        if elements.is_empty() {
            writeln!(out, "}};");
        } else {
            writeln!(out, "");
            out.indent(1);
            write_schema_object_fields(module, out, elements, None, true, false);
            out.dedent(1);
            writeln!(out, "}};");
        }
    }
    writeln!(out, "");

    // --- Procedure builders ---
    let procs: Vec<_> = iter_procedures(module, options.visibility).collect();
    if !procs.is_empty() {
        writeln!(out, "// Procedure builders");
        for procedure in &procs {
            let proc_name = format!("{}Proc", procedure.accessor_name.deref().to_case(Case::Pascal));
            let elements = &procedure.params_for_generate.elements;

            write!(out, "const {}_params = {{", proc_name);
            if elements.is_empty() {
                writeln!(out, "}};");
            } else {
                writeln!(out, "");
                out.indent(1);
                write_schema_object_fields(module, out, elements, None, true, false);
                out.dedent(1);
                writeln!(out, "}};");
            }

            write!(out, "const {}_returnType = ", proc_name);
            write_schema_type_builder(module, out, &procedure.return_type_for_generate).unwrap();
            writeln!(out, ";");
        }
        writeln!(out, "");
    }

    // --- REMOTE_MODULE assembly ---
    writeln!(out, "// Schema assembly");
    writeln!(out, "const tablesSchema = schema({{");
    out.indent(1);
    for table in iter_tables(module, options.visibility) {
        let type_ref = table.product_type_ref;
        let row_name = format!("{}Row", table.accessor_name.deref().to_case(Case::Pascal));

        writeln!(out, "{}: table({{", table.accessor_name);
        out.indent(1);
        write_schema_table_opts(
            module,
            out,
            type_ref,
            &table.name,
            iter_indexes(table),
            iter_constraints(table),
            table.is_event,
        );
        out.dedent(1);
        writeln!(out, "}}, {}),", row_name);
    }
    for view in iter_views(module) {
        let type_ref = view.product_type_ref;
        let row_name = format!("{}Row", view.accessor_name.deref().to_case(Case::Pascal));

        writeln!(out, "{}: table({{", view.accessor_name);
        out.indent(1);
        write_schema_table_opts(module, out, type_ref, &view.name, iter::empty(), iter::empty(), false);
        out.dedent(1);
        writeln!(out, "}}, {}),", row_name);
    }
    out.dedent(1);
    writeln!(out, "}});");
    writeln!(out, "");

    writeln!(out, "const reducersSchema = reducers(");
    out.indent(1);
    for reducer in iter_reducers(module, options.visibility) {
        if !is_reducer_invokable(reducer) {
            continue;
        }
        let args_name = format!("{}Args", reducer.accessor_name.deref().to_case(Case::Pascal));
        writeln!(out, "reducerSchema(\"{}\", {}),", reducer.name, args_name);
    }
    out.dedent(1);
    writeln!(out, ");");
    writeln!(out, "");

    writeln!(out, "const proceduresSchema = procedures(");
    out.indent(1);
    for procedure in &procs {
        let proc_name = format!("{}Proc", procedure.accessor_name.deref().to_case(Case::Pascal));
        writeln!(
            out,
            "procedureSchema(\"{}\", {}_params, {}_returnType),",
            procedure.name, proc_name, proc_name,
        );
    }
    out.dedent(1);
    writeln!(out, ");");
    writeln!(out, "");

    writeln!(out, "export const REMOTE_MODULE = {{");
    out.indent(1);
    writeln!(out, "versionInfo: {{");
    out.indent(1);
    writeln!(out, "cliVersion: \"{}\",", spacetimedb_lib_version());
    out.dedent(1);
    writeln!(out, "}},");
    writeln!(out, "tables: tablesSchema.schemaType.tables,");
    writeln!(out, "reducers: reducersSchema.reducersType.reducers,");
    writeln!(out, "...proceduresSchema,");
    out.dedent(1);
    writeln!(out, "}};");

    OutputFile {
        filename: "StdbSchema.mjs".to_string(),
        code: output.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// StdbBindings.res — Pure ReScript @get/@send externals for SDK interop.
//
// Generates typed bindings for:
// - DB access chain (connection → db → table)
// - Per-table: iter, onInsert/onUpdate/onDelete, isAlive, PK index
// - WS reducers via @send on opaque reducers type
//
// Zero JS generated. All type safety enforced by the ReScript compiler.
// ---------------------------------------------------------------------------

fn generate_bindings_file(module: &ModuleDef, options: &CodegenOptions) -> OutputFile {
    let mut output = CodeIndenter::new(String::new(), INDENT);
    let out = &mut output;

    print_auto_generated_file_comment(out);
    writeln!(out, "");

    // ── Opaque SDK types ──
    writeln!(out, "// Opaque SDK types");
    writeln!(out, "type connection");
    writeln!(out, "type eventCtx");
    writeln!(out, "type db");
    writeln!(out, "type reducers");
    writeln!(out, "");

    // ── Per-table opaque accessor types ──
    writeln!(out, "// Table accessor types");
    let tables: Vec<_> = iter_tables(module, options.visibility).collect();
    for table in &tables {
        let camel = rescript_field_name(table.accessor_name.deref().to_case(Case::Camel));
        writeln!(out, "type {camel}Table");
    }
    writeln!(out, "");

    // ── DB access chain ──
    writeln!(out, "// DB access chain");
    writeln!(out, "@get external db: connection => db = \"db\"");
    writeln!(out, "@get external reducers: connection => reducers = \"reducers\"");
    writeln!(out, "");

    // ── Per-table bindings ──
    for table in &tables {
        let accessor = table.accessor_name.deref();
        let camel = rescript_field_name(accessor.to_case(Case::Camel));
        let pascal = accessor.to_case(Case::Pascal);

        let type_ref = table.product_type_ref;
        let product_def = module.typespace_for_generate()[type_ref].as_product().unwrap();

        // Type name in StdbTypes: table accessor in camelCase (matches generate_types_file naming)
        let type_name = rescript_type_name(accessor.to_case(Case::Pascal));
        let qualified_type = format!("StdbTypes.{type_name}");

        writeln!(out, "// ── {pascal} ──");

        // Table accessor: db → table
        writeln!(out, "@get external {camel}: db => {camel}Table = \"{camel}\"");

        // iter: table → Iterator.t<row>
        writeln!(
            out,
            "@send external iter{pascal}: {camel}Table => Iterator.t<{qualified_type}> = \"iter\""
        );

        // onInsert / onUpdate / onDelete
        writeln!(
            out,
            "@send external on{pascal}Insert: ({camel}Table, (eventCtx, {qualified_type}) => unit) => unit = \"onInsert\""
        );
        writeln!(
            out,
            "@send external on{pascal}Update: ({camel}Table, (eventCtx, {qualified_type}, {qualified_type}) => unit) => unit = \"onUpdate\""
        );
        writeln!(
            out,
            "@send external on{pascal}Delete: ({camel}Table, (eventCtx, {qualified_type}) => unit) => unit = \"onDelete\""
        );

        // isAlive: check if table has a deleted_at field (soft-delete pattern)
        let has_deleted_at = product_def
            .elements
            .iter()
            .any(|(field_name, _)| field_name.deref() == "deleted_at");

        if has_deleted_at {
            writeln!(
                out,
                "let is{pascal}Alive = (row: {qualified_type}) => row.deletedAt->Option.isNone"
            );
        }

        // PK index: if table has a primary key, emit index type + accessor + find
        if let Some(pk_col) = table.primary_key {
            let (pk_field, pk_type) = &product_def.elements[pk_col.idx()];
            let pk_field_camel = rescript_field_name(pk_field.deref().to_case(Case::Camel));

            // Emit PK index type
            writeln!(
                out,
                "type {camel}{pascal_pk}Index",
                pascal_pk = pk_field_camel.to_case(Case::Pascal)
            );

            // Emit @get accessor for the index
            writeln!(
                out,
                "@get external {camel}{pascal_pk}: {camel}Table => {camel}{pascal_pk}Index = \"{pk_field_camel}\"",
                pascal_pk = pk_field_camel.to_case(Case::Pascal)
            );

            // Emit @send find on the index
            write!(
                out,
                "@send external find{pascal}: ({camel}{}Index, ",
                pk_field_camel.to_case(Case::Pascal)
            );
            write_res_type_ctx(module, out, pk_type, false);
            writeln!(out, ") => Nullable.t<{qualified_type}> = \"find\"");
        }

        writeln!(out, "");
    }

    // ── WS Reducer arg types ──
    // ReScript disallows inline record types in external declarations,
    // so we emit named types first, then reference them in the externals.
    writeln!(out, "// ── WS Reducer arg types ──");
    let reducers: Vec<_> = iter_reducers(module, options.visibility)
        .filter(|r| is_reducer_invokable(r))
        .collect();

    for reducer in &reducers {
        let elements = &reducer.params_for_generate.elements;
        if elements.is_empty() {
            continue;
        }
        let accessor = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
        let type_name = format!("{accessor}Args");
        writeln!(out, "type {type_name} = {{");
        out.indent(1);
        for (field, ty) in elements.iter() {
            let field_name = rescript_field_name(field.deref().to_case(Case::Camel));
            write!(out, "{field_name}: ");
            write_res_type_ctx(module, out, ty, false);
            writeln!(out, ",");
        }
        out.dedent(1);
        writeln!(out, "}}");
    }
    writeln!(out, "");

    // ── WS Reducer externals ──
    writeln!(out, "// ── WS Reducers ──");
    for reducer in &reducers {
        let accessor = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
        let elements = &reducer.params_for_generate.elements;

        if elements.is_empty() {
            writeln!(
                out,
                "@send external {accessor}: reducers => promise<unit> = \"{accessor}\""
            );
        } else {
            let type_name = format!("{accessor}Args");
            writeln!(
                out,
                "@send external {accessor}: (reducers, {type_name}) => promise<unit> = \"{accessor}\""
            );
        }
    }

    OutputFile {
        filename: "StdbBindings.res".to_string(),
        code: output.into_inner(),
    }
}

/// Emit `const TypeName = t.object("TypeName", { ... });`
fn write_schema_object_builder(
    module: &ModuleDef,
    out: &mut Indenter,
    name: &str,
    elements: &[(Identifier, AlgebraicTypeUse)],
) {
    write!(out, "const {name} = t.object(\"{name}\", {{");
    if elements.is_empty() {
        writeln!(out, "}});");
    } else {
        writeln!(out, "");
        out.indent(1);
        write_schema_object_fields(module, out, elements, None, true, false);
        out.dedent(1);
        writeln!(out, "}});");
    }
}

/// Emit `const TypeName = t.enum("TypeName", { Variant: t.unit(), ... });`
fn write_schema_enum_builder(
    module: &ModuleDef,
    out: &mut Indenter,
    name: &str,
    variants: &[(Identifier, AlgebraicTypeUse)],
) {
    writeln!(out, "const {name} = t.enum(\"{name}\", {{");
    // Variant names in PascalCase (matching Rust enum variants)
    out.indent(1);
    for (variant_name, variant_type) in variants {
        let pascal = variant_name.deref().to_case(Case::Pascal);
        write_schema_type_builder_field(module, out, &pascal, None, variant_type, false);
    }
    out.dedent(1);
    writeln!(out, "}});");
}

/// Emit object/row fields with `get` getters for forward-referenced Ref types.
fn write_schema_object_fields(
    module: &ModuleDef,
    out: &mut Indenter,
    elements: &[(Identifier, AlgebraicTypeUse)],
    primary_key: Option<ColId>,
    convert_case: bool,
    write_original_name: bool,
) {
    for (i, (ident, ty)) in elements.iter().enumerate() {
        let name = if convert_case {
            ident.deref().to_case(Case::Camel)
        } else {
            ident.deref().into()
        };

        let is_primary_key = match primary_key {
            Some(pk) => pk.idx() == i,
            None => false,
        };
        let original_name = (write_original_name && convert_case && *name != **ident).then_some(&**ident);
        write_schema_type_builder_field(module, out, &name, original_name, ty, is_primary_key);
    }
}

/// Emit a single field, using `get` getter when the type references another type builder
/// (forward-reference pattern — valid JS, required for circular type graphs).
fn write_schema_type_builder_field(
    module: &ModuleDef,
    out: &mut Indenter,
    name: &str,
    original_name: Option<&str>,
    ty: &AlgebraicTypeUse,
    is_primary_key: bool,
) {
    let needs_getter = match ty {
        AlgebraicTypeUse::Ref(_) => true,
        AlgebraicTypeUse::Option(inner) | AlgebraicTypeUse::Array(inner) => {
            matches!(inner.as_ref(), AlgebraicTypeUse::Ref(_))
        }
        AlgebraicTypeUse::Result { ok_ty, err_ty } => {
            matches!(ok_ty.as_ref(), AlgebraicTypeUse::Ref(_)) || matches!(err_ty.as_ref(), AlgebraicTypeUse::Ref(_))
        }
        _ => false,
    };

    if needs_getter {
        writeln!(out, "get {name}() {{");
        out.indent(1);
        write!(out, "return ");
    } else {
        write!(out, "{name}: ");
    }
    write_schema_type_builder(module, out, ty).unwrap();
    if is_primary_key {
        write!(out, ".primaryKey()");
    }
    if let Some(original_name) = original_name {
        write!(out, ".name(\"{original_name}\")");
    }
    if needs_getter {
        writeln!(out, ";");
        out.dedent(1);
        writeln!(out, "}},");
    } else {
        writeln!(out, ",");
    }
}

/// Emit `t.u64()`, `t.option(t.string())`, `TypeName` (for Ref), etc.
fn write_schema_type_builder<W: Write>(module: &ModuleDef, out: &mut W, ty: &AlgebraicTypeUse) -> fmt::Result {
    match ty {
        AlgebraicTypeUse::Unit => write!(out, "t.unit()")?,
        AlgebraicTypeUse::Never => write!(out, "t.never()")?,
        AlgebraicTypeUse::Identity => write!(out, "t.identity()")?,
        AlgebraicTypeUse::ConnectionId => write!(out, "t.connectionId()")?,
        AlgebraicTypeUse::Timestamp => write!(out, "t.timestamp()")?,
        AlgebraicTypeUse::TimeDuration => write!(out, "t.timeDuration()")?,
        AlgebraicTypeUse::ScheduleAt => write!(out, "t.scheduleAt()")?,
        AlgebraicTypeUse::Uuid => write!(out, "t.uuid()")?,
        AlgebraicTypeUse::Option(inner_ty) => {
            write!(out, "t.option(")?;
            write_schema_type_builder(module, out, inner_ty)?;
            write!(out, ")")?;
        }
        AlgebraicTypeUse::Result { ok_ty, err_ty } => {
            write!(out, "t.result(")?;
            write_schema_type_builder(module, out, ok_ty)?;
            write!(out, ", ")?;
            write_schema_type_builder(module, out, err_ty)?;
            write!(out, ")")?;
        }
        AlgebraicTypeUse::Primitive(prim) => match prim {
            PrimitiveType::Bool => write!(out, "t.bool()")?,
            PrimitiveType::I8 => write!(out, "t.i8()")?,
            PrimitiveType::U8 => write!(out, "t.u8()")?,
            PrimitiveType::I16 => write!(out, "t.i16()")?,
            PrimitiveType::U16 => write!(out, "t.u16()")?,
            PrimitiveType::I32 => write!(out, "t.i32()")?,
            PrimitiveType::U32 => write!(out, "t.u32()")?,
            PrimitiveType::I64 => write!(out, "t.i64()")?,
            PrimitiveType::U64 => write!(out, "t.u64()")?,
            PrimitiveType::I128 => write!(out, "t.i128()")?,
            PrimitiveType::U128 => write!(out, "t.u128()")?,
            PrimitiveType::I256 => write!(out, "t.i256()")?,
            PrimitiveType::U256 => write!(out, "t.u256()")?,
            PrimitiveType::F32 => write!(out, "t.f32()")?,
            PrimitiveType::F64 => write!(out, "t.f64()")?,
        },
        AlgebraicTypeUse::String => write!(out, "t.string()")?,
        AlgebraicTypeUse::Array(elem_ty) => {
            if matches!(&**elem_ty, AlgebraicTypeUse::Primitive(PrimitiveType::U8)) {
                return write!(out, "t.byteArray()");
            }
            write!(out, "t.array(")?;
            write_schema_type_builder(module, out, elem_ty)?;
            write!(out, ")")?;
        }
        AlgebraicTypeUse::Ref(r) => {
            write!(out, "{}", type_ref_name(module, *r))?;
        }
    }
    Ok(())
}

/// Emit table options: name, indexes, constraints, event flag.
fn write_schema_table_opts<'a>(
    module: &ModuleDef,
    out: &mut Indenter,
    type_ref: spacetimedb_lib::sats::AlgebraicTypeRef,
    name: &Identifier,
    indexes: impl Iterator<Item = &'a IndexDef>,
    constraints: impl Iterator<Item = &'a ConstraintDef>,
    is_event: bool,
) {
    let product_def = module.typespace_for_generate()[type_ref].as_product().unwrap();
    writeln!(out, "name: '{}',", name.deref());
    writeln!(out, "indexes: [");
    out.indent(1);
    for index_def in indexes {
        if index_def.generated() {
            continue;
        }
        let columns = index_def.algorithm.columns();
        let get_name = |col_pos: spacetimedb_primitives::ColId| {
            let (field_name, _) = &product_def.elements[col_pos.idx()];
            field_name.deref().to_case(Case::Camel)
        };
        if let Some(accessor_name) = &index_def.accessor_name {
            writeln!(out, "{{ name: '{}', algorithm: 'btree', columns: [", accessor_name);
        } else {
            writeln!(out, "{{ name: '{}', algorithm: 'btree', columns: [", index_def.name);
        }
        out.indent(1);
        for col_id in columns.iter() {
            writeln!(out, "'{}',", get_name(col_id));
        }
        out.dedent(1);
        writeln!(out, "] }},");
    }
    out.dedent(1);
    writeln!(out, "],");
    writeln!(out, "constraints: [");
    out.indent(1);
    for constraint in constraints {
        let columns: Vec<_> = constraint
            .data
            .unique_columns()
            .into_iter()
            .flat_map(|cs| cs.iter())
            .map(|col_id| {
                let (field_name, _) = &product_def.elements[col_id.idx()];
                format!("'{}'", field_name.deref().to_case(Case::Camel))
            })
            .collect();

        writeln!(
            out,
            "{{ name: '{}', constraint: 'unique', columns: [{}] }},",
            constraint.name,
            columns.join(", ")
        );
    }
    out.dedent(1);
    writeln!(out, "],");
    if is_event {
        writeln!(out, "event: true,");
    }
}
