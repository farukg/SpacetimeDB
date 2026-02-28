//! Server-side reducer wrappers codegen — generates `StdbServerReducers.res`.

use super::helpers::{reducer_module_name, rescript_field_name};
use super::INDENT;
use crate::code_indenter::CodeIndenter;

use crate::util::{is_reducer_invokable, iter_reducers, print_auto_generated_file_comment};
use crate::{CodegenOptions, OutputFile};

use convert_case::{Case, Casing};
use spacetimedb_schema::def::ModuleDef;
use std::ops::Deref;

pub(super) fn generate_server_reducers_file(module: &ModuleDef, options: &CodegenOptions) -> OutputFile {
    let mut output = CodeIndenter::new(String::new(), INDENT);
    let out = &mut output;

    print_auto_generated_file_comment(out);
    writeln!(out, "");
    writeln!(
        out,
        "// Server-side reducer wrappers — typed, async, with connection management."
    );
    writeln!(
        out,
        "// Each function resolves the server connection, calls the reducer, and"
    );
    writeln!(out, "// optionally waits for sync delay.");
    writeln!(out, "");

    // setTimeout external — not in @rescript/core
    writeln!(
        out,
        "@val external setTimeout: (unit => unit, int) => float = \"setTimeout\""
    );
    writeln!(out, "");

    // Sync delay from environment
    writeln!(out, "@val @scope((\"process\", \"env\"))");
    writeln!(
        out,
        "external syncDelayMsEnv: option<string> = \"STDB_TYPED_REDUCER_SYNC_DELAY_MS\""
    );
    writeln!(out, "");
    writeln!(out, "let syncDelayMs = switch syncDelayMsEnv {{");
    writeln!(out, "| Some(s) => Int.fromString(s)->Option.getOr(300)");
    writeln!(out, "| None => 300");
    writeln!(out, "}}");
    writeln!(out, "");

    // sleep helper
    writeln!(out, "let sleep = (ms) => {{");
    writeln!(out, "  Promise.make((resolve, _reject) => {{");
    writeln!(out, "    let _ = setTimeout(() => resolve(), ms)");
    writeln!(out, "  }})");
    writeln!(out, "}}");

    let invokable_reducers: Vec<_> = iter_reducers(module, options.visibility)
        .filter(|reducer| is_reducer_invokable(reducer))
        .collect();

    // Per-reducer typed wrappers.
    // Each wrapper calls the @send external defined in the reducer module,
    // qualified as {ReducerModule}.{accessor} so ReScript resolves it correctly.
    for reducer in &invokable_reducers {
        let reducer_module = reducer_module_name(&reducer.name);
        let reducer_name_camel = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
        let has_args = !reducer.params_for_generate.elements.is_empty();

        writeln!(out, "");
        if has_args {
            writeln!(
                out,
                "let {reducer_name_camel} = async (args: {reducer_module}.args) => {{"
            );
        } else {
            writeln!(out, "let {reducer_name_camel} = async () => {{");
        }
        out.indent(1);
        writeln!(
            out,
            "let conn: StdbTypes.connection = Obj.magic(await StdbServerConnection.getConnection())"
        );
        if has_args {
            writeln!(
                out,
                "let result = await conn->StdbClient.reducers->{reducer_module}.{reducer_name_camel}(args)"
            );
        } else {
            writeln!(
                out,
                "let result = await conn->StdbClient.reducers->{reducer_module}.{reducer_name_camel}"
            );
        }
        writeln!(out, "if syncDelayMs > 0 {{ await sleep(syncDelayMs) }}");
        writeln!(out, "result");
        out.dedent(1);
        writeln!(out, "}}");
    }

    // Typed record of all server reducers
    if !invokable_reducers.is_empty() {
        writeln!(out, "");
        writeln!(out, "type serverReducers = {{");
        out.indent(1);
        for reducer in &invokable_reducers {
            let reducer_module = reducer_module_name(&reducer.name);
            let reducer_name_camel = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
            let has_args = !reducer.params_for_generate.elements.is_empty();

            if has_args {
                writeln!(out, "{reducer_name_camel}: {reducer_module}.args => promise<unit>,");
            } else {
                writeln!(out, "{reducer_name_camel}: unit => promise<unit>,");
            }
        }
        out.dedent(1);
        writeln!(out, "}}");
        writeln!(out, "");
        writeln!(out, "let serverReducers: serverReducers = {{");
        out.indent(1);
        for reducer in &invokable_reducers {
            let reducer_name_camel = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
            writeln!(out, "{reducer_name_camel}: {reducer_name_camel},");
        }
        out.dedent(1);
        writeln!(out, "}}");
    }

    OutputFile {
        filename: "StdbServerReducers.res".to_string(),
        code: output.into_inner(),
    }
}
