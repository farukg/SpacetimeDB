use spacetimedb_codegen::{generate, CodegenError, CodegenOptions, Csharp, Lang, OutputFile, Rust, TypeScript};
use spacetimedb_lib::db::raw_def::v10::RawModuleDefV10Builder;
use spacetimedb_lib::db::raw_def::v9::TableAccess;
use spacetimedb_lib::sats::AlgebraicTypeRef;
use spacetimedb_lib::{AlgebraicType, ProductType};
use spacetimedb_schema::def::{ModuleDef, ProcedureDef, ReducerDef, TableDef, TypeDef};
use spacetimedb_schema::schema::TableSchema;
use std::collections::HashSet;

fn library() -> spacetimedb_lib::db::raw_def::v10::RawModuleDefV10 {
    let mut builder = RawModuleDefV10Builder::new();
    let row = builder
        .build_table_with_new_type("row", [("id", AlgebraicType::U64)], true)
        .with_access(TableAccess::Public)
        .finish();
    builder.add_view(
        "visible",
        0,
        true,
        false,
        ProductType::unit(),
        AlgebraicType::array(AlgebraicType::Ref(row)),
    );
    builder.add_reducer("store", ProductType::unit());
    builder.add_procedure("read", ProductType::unit(), AlgebraicType::U64);
    builder.finish()
}

fn nested_module() -> ModuleDef {
    let mut child = library();
    let mut nested = RawModuleDefV10Builder::new();
    nested.add_submodule("deep", library());
    child.sections.extend(nested.finish().sections);
    let mut root = RawModuleDefV10Builder::new();
    root.add_submodule("myLib", child);
    root.finish().try_into().expect("valid nested module")
}

#[derive(Clone, Copy)]
enum FailurePoint {
    Rows,
    Reducer,
    Procedure,
}

struct FailingGenerator(FailurePoint);

fn failure() -> CodegenError {
    CodegenError::MissingRowType {
        row_type: AlgebraicTypeRef(999),
    }
}

impl Lang for FailingGenerator {
    fn generate_table_file_from_schema(
        &self,
        module: &ModuleDef,
        table: &TableDef,
        schema: TableSchema,
    ) -> Result<OutputFile, CodegenError> {
        match self.0 {
            FailurePoint::Rows => Err(failure()),
            FailurePoint::Reducer | FailurePoint::Procedure => {
                Rust.generate_table_file_from_schema(module, table, schema)
            }
        }
    }

    fn generate_type_files(&self, module: &ModuleDef, typ: &TypeDef) -> Vec<OutputFile> {
        Rust.generate_type_files(module, typ)
    }

    fn generate_reducer_file(&self, module: &ModuleDef, reducer: &ReducerDef) -> Result<OutputFile, CodegenError> {
        match self.0 {
            FailurePoint::Reducer => Err(failure()),
            FailurePoint::Rows | FailurePoint::Procedure => Rust.generate_reducer_file(module, reducer),
        }
    }

    fn generate_procedure_file(
        &self,
        module: &ModuleDef,
        procedure: &ProcedureDef,
    ) -> Result<OutputFile, CodegenError> {
        match self.0 {
            FailurePoint::Procedure => Err(failure()),
            FailurePoint::Rows | FailurePoint::Reducer => Rust.generate_procedure_file(module, procedure),
        }
    }

    fn generate_global_files(
        &self,
        module: &ModuleDef,
        options: &CodegenOptions,
    ) -> Result<Vec<OutputFile>, CodegenError> {
        Rust.generate_global_files(module, options)
    }
}

fn assert_original_failure(result: Result<impl Sized, CodegenError>) {
    assert!(matches!(
        result,
        Err(CodegenError::MissingRowType { row_type }) if row_type == AlgebraicTypeRef(999)
    ));
}

#[test]
fn submodule_generation_preserves_language_errors() {
    let module = nested_module();
    for point in [FailurePoint::Rows, FailurePoint::Reducer, FailurePoint::Procedure] {
        assert_original_failure(generate(&module, &FailingGenerator(point), &CodegenOptions::default()));
    }
}

#[test]
fn submodule_view_preserves_language_error() {
    let module = nested_module();
    let child = module.submodules().values().next().expect("library");
    let view = child.views().next().expect("library view");
    assert_original_failure(FailingGenerator(FailurePoint::Rows).generate_submodule_view_file(child, view));
}

#[test]
fn nested_language_outputs_use_accessor_namespaces_once() {
    let module = nested_module();
    let csharp = Csharp { namespace: "Client" };
    let typescript = TypeScript::default();
    let languages: [&dyn Lang; 3] = [&Rust, &csharp, &typescript];
    for language in languages {
        let files = generate(&module, language, &CodegenOptions::default()).expect("generated bindings");
        let names: HashSet<_> = files.iter().map(|file| file.filename.as_str()).collect();
        assert_eq!(names.len(), files.len(), "generated file paths collide");
        assert!(names.iter().any(|name| name.starts_with("myLib/")));
        assert!(names.iter().any(|name| name.starts_with("myLib/deep/")));
        assert!(!names
            .iter()
            .any(|name| name.starts_with("my_lib/") || name.contains("myLib/myLib/")));
    }
}
