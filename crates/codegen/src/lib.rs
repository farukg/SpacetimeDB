use spacetimedb_lib::db::raw_def::v9::TableAccess;
use spacetimedb_schema::def::{ModuleDef, ProcedureDef, ReducerDef, TableDef, TypeDef, ViewDef};
use spacetimedb_schema::schema::{Schema, TableSchema};
use spacetimedb_schema::type_for_generate::{AlgebraicTypeDef, AlgebraicTypeUse};

#[cfg(feature = "rescript")]
use self::rescript::DisplayProjectionAdapterError;

mod code_indenter;
pub mod cpp;
pub mod csharp;
#[cfg(feature = "rescript")]
pub mod rescript;
pub mod rust;
pub mod typescript;
pub mod unrealcpp;
mod util;

pub use self::csharp::Csharp;
#[cfg(feature = "rescript")]
pub use self::rescript::ReScript;
pub use self::rust::Rust;
pub use self::typescript::{ImportExtension, TypeScript};
pub use self::unrealcpp::UnrealCpp;
pub use util::private_table_names;
pub use util::CodegenVisibility;
pub use util::AUTO_GENERATED_PREFIX;

#[derive(Clone, Copy, Debug)]
pub struct CodegenOptions {
    pub visibility: CodegenVisibility,
}

impl Default for CodegenOptions {
    fn default() -> Self {
        Self {
            visibility: CodegenVisibility::OnlyPublic,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    #[error("table schema validation failed: {0:?}")]
    TableSchema(Vec<spacetimedb_schema::def::error::SchemaError>),
    #[error("row type definition {row_type:?} is absent from the generation typespace")]
    MissingRowType {
        row_type: spacetimedb_lib::sats::AlgebraicTypeRef,
    },
    #[error("row type definition {row_type:?} must be a product, found {actual}")]
    RowTypeNotProduct {
        row_type: spacetimedb_lib::sats::AlgebraicTypeRef,
        actual: RowTypeDefinition,
    },
    #[cfg(feature = "rescript")]
    #[error(transparent)]
    DisplayProjection(#[from] DisplayProjectionAdapterError),
    /// ReScript representation / config validation failed before write.
    #[cfg(feature = "rescript")]
    #[error("{0}")]
    Rescript(String),
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum RowTypeDefinition {
    #[error("sum")]
    Sum,
    #[error("plain enum")]
    PlainEnum,
}

pub fn row_product_definition(
    module: &ModuleDef,
    row_type: spacetimedb_lib::sats::AlgebraicTypeRef,
) -> Result<&[(spacetimedb_schema::identifier::Identifier, AlgebraicTypeUse)], CodegenError> {
    match module.typespace_for_generate().get(row_type) {
        None => Err(CodegenError::MissingRowType { row_type }),
        Some(AlgebraicTypeDef::Product(product)) => Ok(&product.elements),
        Some(AlgebraicTypeDef::Sum(_)) => Err(CodegenError::RowTypeNotProduct {
            row_type,
            actual: RowTypeDefinition::Sum,
        }),
        Some(AlgebraicTypeDef::PlainEnum(_)) => Err(CodegenError::RowTypeNotProduct {
            row_type,
            actual: RowTypeDefinition::PlainEnum,
        }),
    }
}

pub fn generate(
    module: &ModuleDef,
    lang: &dyn Lang,
    options: &CodegenOptions,
) -> Result<Vec<OutputFile>, CodegenError> {
    itertools::chain!(
        lang.generate_table_files(module, options)?.into_iter().map(Ok),
        module.views().map(|view| lang.generate_view_file(module, view)),
        // Public tables from submodules
        module
            .all_tables_with_prefix()
            .into_iter()
            .filter(|(prefix, _, table)| !prefix.is_empty() && table.table_access == TableAccess::Public)
            .map(|(_, owning_def, table)| lang.generate_submodule_table_file(owning_def, table)),
        // Views from submodules (views are currently always public)
        module
            .all_views_with_prefix()
            .into_iter()
            .filter(|(prefix, _, _)| !prefix.is_empty())
            .map(|(_, owning_def, view)| lang.generate_submodule_view_file(owning_def, view)),
        module
            .types()
            .flat_map(|typ| lang.generate_type_files(module, typ).into_iter().map(Ok)),
        util::iter_reducers(module, options.visibility).map(|reducer| lang.generate_reducer_file(module, reducer)),
        // Reducers from submodules
        module
            .all_reducers_with_prefix()
            .into_iter()
            .filter(|(prefix, _, reducer)| !prefix.is_empty() && !reducer.visibility.is_private())
            .map(|(_, owning_def, reducer)| lang.generate_submodule_reducer_file(owning_def, reducer)),
        {
            // Procedure and global owners may share generation-local schema analysis.
            match lang.generate_procedure_and_global_files(module, options) {
                Ok(files) => files.into_iter().map(Ok).collect::<Vec<_>>(),
                Err(err) => vec![Err(err)],
            }
        }
        .into_iter(),
    )
    .collect()
}

pub struct OutputFile {
    pub filename: String,
    pub code: String,
}

impl OutputFile {
    fn in_namespace(mut self, namespace: &spacetimedb_schema::identifier::NamespacePath) -> Self {
        self.filename = format!("{}/{}", namespace.join_segments("/"), self.filename);
        self
    }
}

pub trait Lang {
    fn generate_procedure_and_global_files(
        &self,
        module: &ModuleDef,
        options: &CodegenOptions,
    ) -> Result<Vec<OutputFile>, CodegenError> {
        let mut files = util::iter_procedures(module, options.visibility)
            .map(|procedure| self.generate_procedure_file(module, procedure))
            .collect::<Result<Vec<_>, _>>()?;
        for (prefix, owner, procedure) in module.all_procedures_with_prefix() {
            if !prefix.is_empty() && !procedure.visibility.is_private() {
                files.push(self.generate_submodule_procedure_file(owner, procedure)?);
            }
        }
        files.extend(self.generate_global_files(module, options)?);
        Ok(files)
    }

    fn generate_table_file_from_schema(
        &self,
        module: &ModuleDef,
        tbl: &TableDef,
        schema: TableSchema,
    ) -> Result<OutputFile, CodegenError>;
    fn generate_type_files(&self, module: &ModuleDef, typ: &TypeDef) -> Vec<OutputFile>;
    fn generate_reducer_file(&self, module: &ModuleDef, reducer: &ReducerDef) -> Result<OutputFile, CodegenError>;
    fn generate_procedure_file(&self, module: &ModuleDef, procedure: &ProcedureDef)
        -> Result<OutputFile, CodegenError>;
    fn generate_global_files(
        &self,
        module: &ModuleDef,
        options: &CodegenOptions,
    ) -> Result<Vec<OutputFile>, CodegenError>;

    fn generate_table_files(
        &self,
        module: &ModuleDef,
        options: &CodegenOptions,
    ) -> Result<Vec<OutputFile>, CodegenError> {
        util::iter_tables(module, options.visibility)
            .map(|table| self.generate_table_file(module, table))
            .collect()
    }

    fn generate_table_file(&self, module: &ModuleDef, tbl: &TableDef) -> Result<OutputFile, CodegenError> {
        let schema = TableSchema::from_module_def(module, tbl, (), 0.into())
            .validated()
            .map_err(CodegenError::TableSchema)?;
        self.generate_table_file_from_schema(module, tbl, schema)
    }

    fn generate_view_file(&self, module: &ModuleDef, view: &ViewDef) -> Result<OutputFile, CodegenError> {
        let tbl = TableDef::from(view.clone());
        let schema = TableSchema::from_view_def_for_codegen(module, view)
            .validated()
            .map_err(CodegenError::TableSchema)?;
        self.generate_table_file_from_schema(module, &tbl, schema)
    }

    /// Generate a row-type file for a public table from a submodule.
    /// Uses `owning_def`'s typespace for type resolution.
    /// Filename goes in a subdirectory named after the *accessor* namespace path:
    /// e.g. `myLib/table_name_table.ts` for a submodule mounted as `myLib`, table `tableName`.
    fn generate_submodule_table_file(
        &self,
        owning_def: &ModuleDef,
        table: &TableDef,
    ) -> Result<OutputFile, CodegenError> {
        self.generate_table_file(owning_def, table)
            .map(|file| file.in_namespace(owning_def.accessor_path()))
    }

    /// Generate a row-type file for a view from a submodule.
    /// Filename goes in a subdirectory named after the accessor namespace path.
    fn generate_submodule_view_file(&self, owning_def: &ModuleDef, view: &ViewDef) -> Result<OutputFile, CodegenError> {
        self.generate_view_file(owning_def, view)
            .map(|file| file.in_namespace(owning_def.accessor_path()))
    }

    /// Generate an arg-schema file for a reducer from a submodule.
    /// Filename goes in a subdirectory named after the accessor namespace path.
    fn generate_submodule_reducer_file(
        &self,
        owning_def: &ModuleDef,
        reducer: &ReducerDef,
    ) -> Result<OutputFile, CodegenError> {
        self.generate_reducer_file(owning_def, reducer)
            .map(|file| file.in_namespace(owning_def.accessor_path()))
    }

    /// Generate an arg-schema file for a procedure from a submodule.
    /// Filename goes in a subdirectory named after the accessor namespace path.
    fn generate_submodule_procedure_file(
        &self,
        owning_def: &ModuleDef,
        procedure: &ProcedureDef,
    ) -> Result<OutputFile, CodegenError> {
        self.generate_procedure_file(owning_def, procedure)
            .map(|file| file.in_namespace(owning_def.accessor_path()))
    }
}
