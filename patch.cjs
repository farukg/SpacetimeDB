const fs = require("fs");
const file = "/home/fg/projects/forks/SpacetimeDB/crates/codegen/src/rescript.rs";
let code = fs.readFileSync(file, "utf8");

code = code.replace(
    "fn generate_global_files(&self, module: &ModuleDef, options: &CodegenOptions) -> Vec<OutputFile> {\n        vec![generate_types_file(module), generate_index_file(module, options)]\n    }",
    "fn generate_global_files(&self, module: &ModuleDef, options: &CodegenOptions) -> Vec<OutputFile> {\n        vec![generate_types_file(module), generate_index_file(module, options), generate_reducers_file(module, options)]\n    }"
);

const newFunc = `
fn generate_encoder(module: &ModuleDef, ty: &AlgebraicTypeUse, val: &str) -> String {
    match ty {
        AlgebraicTypeUse::Option(inner) => {
            let inner_encoder = generate_encoder(module, inner, "v");
            format!("encodeOption({val}, v => {inner_encoder})")
        }
        AlgebraicTypeUse::Array(inner) => {
            let inner_encoder = generate_encoder(module, inner, "v");
            format!("asJson({val}->Js.Array2.map(v => {inner_encoder}))")
        }
        AlgebraicTypeUse::Ref(type_ref) => {
            let def = &module.typespace_for_generate()[*type_ref];
            if def.as_sum().is_some() {
                format!("encodeEnum(Obj.magic({val}))")
            } else {
                format!("encodeIdentity({val})")
            }
        }
        _ => format!("encodeIdentity({val})"),
    }
}

fn generate_reducers_file(module: &ModuleDef, options: &CodegenOptions) -> OutputFile {
    let mut output = CodeIndenter::new(String::new(), INDENT);
    let out = &mut output;

    print_auto_generated_file_comment(out);
    writeln!(out, "");

    writeln!(out, "external asJson: 'a => JSON.t = \\"%identity\\\"");
    writeln!(out, "@module(\\"../../api/stdb-server.mjs\\")");
    writeln!(out, "external callReducerRaw: (string, 'a) => promise<bool> = \\"callReducerRaw\\"");
    writeln!(out, "");
    writeln!(out, "let encodeOption = (opt, encoder) => {{");
    writeln!(out, "  switch opt {{");
    writeln!(out, "  | Some(v) => asJson({{\\"some\\": encoder(v)}})");
    writeln!(out, "  | None => asJson(Js.Nullable.null)");
    writeln!(out, "  }}");
    writeln!(out, "}}");
    writeln!(out, "");
    writeln!(out, "let encodeEnum = (v) => asJson({{\\"tag\\": v}})");
    writeln!(out, "let encodeIdentity = (v) => asJson(v)");
    writeln!(out, "");

    for reducer in iter_reducers(module, options.visibility) {
        let fn_name = rescript_field_name(reducer.accessor_name.deref().to_case(Case::Camel));
        let mut params = Vec::new();
        let mut encoders = Vec::new();

        for arg in &reducer.params_for_generate.elements {
            let arg_name = rescript_field_name(arg.name.to_case(Case::Camel));
            let mut arg_type = String::new();
            {
                let mut type_out = CodeIndenter::new(String::new(), INDENT);
                write_res_type_ctx(module, &mut type_out, &arg.algebraic_type, false);
                arg_type = type_out.into_inner();
            }
            let arg_type = arg_type.trim();

            params.push(format!("~{arg_name}: {arg_type}"));
            
            let encoded_val = generate_encoder(module, &arg.algebraic_type, &arg_name);
            encoders.push(encoded_val);
        }

        if params.is_empty() {
            writeln!(out, "let {fn_name} = () => {{");
        } else {
            writeln!(out, "let {fn_name} = ({}) => {{", params.join(", "));
        }
        out.indent(1);
        
        let reducer_string_name = &reducer.name;
        if encoders.is_empty() {
            writeln!(out, "callReducerRaw(\\"{reducer_string_name}\", [])");
        } else {
            writeln!(out, "callReducerRaw(\\"{reducer_string_name}\", (");
            out.indent(1);
            for enc in encoders {
                writeln!(out, "{enc},");
            }
            out.dedent(1);
            writeln!(out, "))");
        }
        out.dedent(1);
        writeln!(out, "}}");
        writeln!(out, "");
    }

    OutputFile {
        filename: "StdbReducers.res".to_string(),
        code: output.into_inner(),
    }
}
`;

code = code.replace("fn generate_types_file(module: &ModuleDef) -> OutputFile {", newFunc + "\nfn generate_types_file(module: &ModuleDef) -> OutputFile {");

fs.writeFileSync(file, code);
