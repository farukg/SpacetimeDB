use crate::util::{
    collect_case, iter_procedures, iter_reducers, iter_table_names_and_types, iter_types,
    print_auto_generated_file_comment,
};
use crate::{CodegenOptions, OutputFile};

use super::code_indenter::{CodeIndenter, Indenter};
use super::util::type_ref_name;
use super::Lang;

use convert_case::{Case, Casing};
use spacetimedb_lib::sats::layout::PrimitiveType;
use spacetimedb_schema::def::{ModuleDef, ProcedureDef, ReducerDef, TableDef, TypeDef};
use spacetimedb_schema::identifier::Identifier;
use spacetimedb_schema::reducer_name::ReducerName;
use spacetimedb_schema::schema::TableSchema;
use spacetimedb_schema::type_for_generate::{AlgebraicTypeDef, AlgebraicTypeUse};
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

    fn generate_reducer_file(&self, module: &ModuleDef, reducer: &ReducerDef) -> OutputFile {
        let mut output = CodeIndenter::new(String::new(), INDENT);
        let out = &mut output;

        print_auto_generated_file_comment(out);
        writeln!(out, "");

        write_record_type(module, out, "params", &reducer.params_for_generate.elements);
        writeln!(out, "let reducerName = \"{}\"", reducer.name);

        OutputFile {
            filename: format!("{}.res", reducer_module_name(&reducer.accessor_name)),
            code: output.into_inner(),
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
        vec![generate_types_file(module), generate_index_file(module, options)]
    }
}

fn generate_types_file(module: &ModuleDef) -> OutputFile {
    let mut output = CodeIndenter::new(String::new(), INDENT);
    let out = &mut output;

    print_auto_generated_file_comment(out);
    writeln!(out, "");

    for ty in iter_types(module) {
        let type_name = rescript_type_name(collect_case(Case::Pascal, ty.accessor_name.name_segments()));
        match &module.typespace_for_generate()[ty.ty] {
            AlgebraicTypeDef::Product(product) => write_record_type(module, out, &type_name, &product.elements),
            AlgebraicTypeDef::Sum(sum) => write_sum_type(module, out, &type_name, &sum.variants),
            AlgebraicTypeDef::PlainEnum(plain_enum) => {
                writeln!(out, "type {type_name} =");
                out.indent(1);
                for variant in &plain_enum.variants {
                    let constructor = rescript_constructor_name(variant.deref());
                    writeln!(out, "| {constructor}");
                }
                out.dedent(1);
                writeln!(out, "");
            }
        }
    }

    OutputFile {
        filename: "Types.res".to_string(),
        code: output.into_inner(),
    }
}

fn generate_index_file(module: &ModuleDef, options: &CodegenOptions) -> OutputFile {
    let mut output = CodeIndenter::new(String::new(), INDENT);
    let out = &mut output;

    print_auto_generated_file_comment(out);
    writeln!(out, "");
    writeln!(out, "module Types = Types");
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

    writeln!(out, "module Reducers = {{");
    out.indent(1);
    for reducer in iter_reducers(module, options.visibility) {
        let alias = reducer.accessor_name.deref().to_case(Case::Pascal);
        let reducer_module = reducer_module_name(&reducer.accessor_name);
        writeln!(out, "module {alias} = {reducer_module}");
    }
    out.dedent(1);
    writeln!(out, "}}");
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

fn write_record_type(module: &ModuleDef, out: &mut Indenter, name: &str, elements: &[(Identifier, AlgebraicTypeUse)]) {
    if elements.is_empty() {
        writeln!(out, "type {name} = unit");
        writeln!(out, "");
        return;
    }

    writeln!(out, "type {name} = {{");
    out.indent(1);
    for (field, ty) in elements {
        let field_name = rescript_field_name(field.deref().to_case(Case::Camel));
        write!(out, "{field_name}: ");
        write_res_type(module, out, ty);
        writeln!(out, ",");
    }
    out.dedent(1);
    writeln!(out, "}}");
    writeln!(out, "");
}

fn write_sum_type(module: &ModuleDef, out: &mut Indenter, name: &str, variants: &[(Identifier, AlgebraicTypeUse)]) {
    writeln!(out, "type {name} =");
    out.indent(1);
    for (variant_name, variant_type) in variants {
        let constructor = rescript_constructor_name(variant_name.deref());
        if matches!(variant_type, AlgebraicTypeUse::Unit) {
            writeln!(out, "| {constructor}");
        } else {
            write!(out, "| {constructor}(");
            write_res_type(module, out, variant_type);
            writeln!(out, ")");
        }
    }
    out.dedent(1);
    writeln!(out, "");
}

fn write_res_type(module: &ModuleDef, out: &mut Indenter, ty: &AlgebraicTypeUse) {
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
            write!(out, "float");
        }
        AlgebraicTypeUse::ScheduleAt => {
            write!(out, "[ #Interval(float) | #Time(float) ]");
        }
        AlgebraicTypeUse::Option(inner) => {
            write!(out, "option<");
            write_res_type(module, out, inner);
            write!(out, ">");
        }
        AlgebraicTypeUse::Result { ok_ty, err_ty } => {
            write!(out, "result<");
            write_res_type(module, out, ok_ty);
            write!(out, ", ");
            write_res_type(module, out, err_ty);
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
            write_res_type(module, out, inner);
            write!(out, ">");
        }
        AlgebraicTypeUse::Ref(reference) => {
            let reference_name = rescript_type_name(type_ref_name(module, *reference));
            write!(out, "Types.{reference_name}");
        }
    }
}

fn table_module_name(table_name: &Identifier) -> String {
    format!("{}Table", table_name.deref().to_case(Case::Pascal))
}

fn reducer_module_name(reducer_name: &ReducerName) -> String {
    format!("{}Reducer", reducer_name.deref().to_case(Case::Pascal))
}

fn procedure_module_name(procedure_name: &Identifier) -> String {
    format!("{}Procedure", procedure_name.deref().to_case(Case::Pascal))
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
    let pascal = name.to_case(Case::Pascal);
    if pascal.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("V{pascal}")
    } else {
        pascal
    }
}
