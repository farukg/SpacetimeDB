use crate::util::{
    is_reducer_invokable, iter_procedures, iter_reducers, iter_table_names_and_types, print_auto_generated_file_comment,
};
use crate::{CodegenOptions, OutputFile};

use super::helpers::{procedure_module_name, reducer_module_name, table_module_name};
use crate::code_indenter::CodeIndenter;

use convert_case::{Case, Casing};
use spacetimedb_schema::def::ModuleDef;
use std::ops::Deref;

pub(super) fn generate_index_file(module: &ModuleDef, options: &CodegenOptions) -> OutputFile {
    let mut output = CodeIndenter::new(String::new(), super::INDENT);
    let out = &mut output;

    print_auto_generated_file_comment(out);
    writeln!(out, "");
    writeln!(out, "module StdbTypes = StdbTypes");
    writeln!(out, "module StdbClient = StdbClient");
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
        if !is_reducer_invokable(reducer) {
            continue;
        }
        let alias = reducer.accessor_name.deref().to_case(Case::Pascal);
        let reducer_module = reducer_module_name(&reducer.name);
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
