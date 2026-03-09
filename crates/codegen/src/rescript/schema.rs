//! StdbSchema.res — pure ReScript runtime schema for the SpacetimeDB SDK.
//!
//! Constructs `remoteModule` directly using `StdbSdk` types and direct
//! algebraicType constructors. No SDK builder functions, no CodeIndenter —
//! all output is composed via boilerplate templates.
//!
//! The generated file:
//! 1. Opens `StdbSdk` for all types
//! 2. Defines `let` bindings for each named type's `algebraicType` (topo-sorted)
//! 3. Builds `tableDef` records inline with `productType`, `columnDef`, etc.
//! 4. Builds `reducerDef` records inline
//! 5. Assembles the `remoteModule` record
//! 6. Exports `tables`, `reducers`, `allTableNames`

use super::helpers::{render_schema_alg_type, rescript_constructor_name, schema_type_binding_name};
use super::templates::{
    AutoGenHeaderRes, SchemaColumnEntryRes, SchemaCompoundBindingRes, SchemaConstraintEntryRes, SchemaIndexEntryRes,
    SchemaNamedElementRes, SchemaProcedureEntryRes, SchemaReducerEntryRes, SchemaTableEntryRes, StdbSchemaRes,
};
use super::topo::{topological_groups, TypeGroup};
use crate::util::{
    collect_case, is_reducer_invokable, iter_constraints, iter_indexes, iter_procedures, iter_reducers, iter_tables,
    iter_types, iter_views,
};
use crate::{CodegenOptions, OutputFile};

use convert_case::{Case, Casing};
use spacetimedb_lib::sats::AlgebraicTypeRef;
use spacetimedb_lib::version::spacetimedb_lib_version;
use spacetimedb_schema::def::ModuleDef;
use spacetimedb_schema::identifier::Identifier;
use spacetimedb_schema::type_for_generate::AlgebraicTypeDef;
use std::collections::HashMap;
use std::ops::Deref;

pub(super) fn generate_schema_file(module: &ModuleDef, options: &CodegenOptions, root_module: &str) -> OutputFile {
    let cli_version = spacetimedb_lib_version();
    let sdk_module = format!("{root_module}__Sdk");

    // ── Named type bindings (topologically sorted — dependencies first) ──
    let types: Vec<_> = iter_types(module).collect();
    // Build AlgebraicTypeRef → TypeDef lookup for topo-sorted iteration
    let ref_to_typedef: HashMap<AlgebraicTypeRef, &_> = types.iter().map(|t| (t.ty, *t)).collect();
    let type_refs: Vec<AlgebraicTypeRef> = types.iter().map(|t| t.ty).collect();
    let groups = topological_groups(module, &type_refs);
    // Flatten topo groups into ordered refs
    let topo_ordered_refs: Vec<AlgebraicTypeRef> = groups
        .into_iter()
        .flat_map(|g| match g {
            TypeGroup::Standalone(r) => vec![r],
            TypeGroup::SelfRecursive(r) => vec![r],
            TypeGroup::MutuallyRecursive(rs) => rs,
        })
        .collect();
    let type_bindings_str = {
        let mut buf = String::new();
        for type_ref in &topo_ordered_refs {
            let ty = ref_to_typedef[type_ref];
            let pascal_name = collect_case(Case::Pascal, ty.accessor_name.name_segments());
            let binding_name = schema_type_binding_name(&pascal_name);

            match &module.typespace_for_generate()[ty.ty] {
                AlgebraicTypeDef::Product(product) => {
                    let elem_data: Vec<_> = product
                        .elements
                        .iter()
                        .map(|(field, field_ty)| {
                            let name = field.deref().to_string();
                            let alg = render_schema_alg_type(module, field_ty);
                            (name, alg)
                        })
                        .collect();
                    let elements: Vec<SchemaNamedElementRes> = elem_data
                        .iter()
                        .map(|(name, alg)| SchemaNamedElementRes {
                            entry_name: name,
                            alg_type_expr: alg,
                        })
                        .collect();
                    buf.push_str(
                        &SchemaCompoundBindingRes {
                            binding_name: &binding_name,
                            is_sum: false,
                            items: elements,
                        }
                        .to_string(),
                    );
                }
                AlgebraicTypeDef::Sum(sum) => {
                    let variant_data: Vec<_> = sum
                        .variants
                        .iter()
                        .map(|(variant_name, variant_ty)| {
                            let name = rescript_constructor_name(variant_name.deref());
                            let alg = render_schema_alg_type(module, variant_ty);
                            (name, alg)
                        })
                        .collect();
                    let variants: Vec<SchemaNamedElementRes> = variant_data
                        .iter()
                        .map(|(name, alg)| SchemaNamedElementRes {
                            entry_name: name,
                            alg_type_expr: alg,
                        })
                        .collect();
                    buf.push_str(
                        &SchemaCompoundBindingRes {
                            binding_name: &binding_name,
                            is_sum: true,
                            items: variants,
                        }
                        .to_string(),
                    );
                }
                AlgebraicTypeDef::PlainEnum(plain_enum) => {
                    // Plain enums are sums with all-unit variants
                    let variant_data: Vec<_> = plain_enum
                        .variants
                        .iter()
                        .map(|variant_name| {
                            let name = rescript_constructor_name(variant_name.deref());
                            let alg = "Compound(Product({value: {elements: []}}))".to_string();
                            (name, alg)
                        })
                        .collect();
                    let variants: Vec<SchemaNamedElementRes> = variant_data
                        .iter()
                        .map(|(name, alg)| SchemaNamedElementRes {
                            entry_name: name,
                            alg_type_expr: alg,
                        })
                        .collect();
                    buf.push_str(
                        &SchemaCompoundBindingRes {
                            binding_name: &binding_name,
                            is_sum: true,
                            items: variants,
                        }
                        .to_string(),
                    );
                }
            }
        }
        buf
    };

    // ── Table entries ────────────────────────────────────────────────
    let table_entry_data: Vec<TableEntryData> = iter_tables(module, options.visibility)
        .map(|table| build_table_entry_data(module, table))
        .collect();

    let view_entry_data: Vec<TableEntryData> = iter_views(module)
        .map(|view| build_table_entry_data(module, view))
        .collect();

    let all_entry_data: Vec<&TableEntryData> = table_entry_data.iter().chain(view_entry_data.iter()).collect();

    let table_entries: Vec<SchemaTableEntryRes> = all_entry_data.iter().map(|d| d.to_template()).collect();

    let all_table_names: Vec<&str> = all_entry_data.iter().map(|d| d.accessor_name.as_str()).collect();

    // Non-event tables/views — safe for client subscription via SELECT * FROM.
    let subscribable_table_names: Vec<&str> = all_entry_data
        .iter()
        .filter(|d| !d.is_event)
        .map(|d| d.accessor_name.as_str())
        .collect();

    // ── Reducer entries ──────────────────────────────────────────────
    let reducer_data: Vec<ReducerEntryData> = iter_reducers(module, options.visibility)
        .filter(|r| is_reducer_invokable(r))
        .map(|reducer| {
            let accessor_name = reducer.accessor_name.deref().to_case(Case::Camel);
            let param_elem_data: Vec<_> = reducer
                .params_for_generate
                .elements
                .iter()
                .map(|(field, field_ty)| {
                    let name = field.deref().to_string();
                    let alg = render_schema_alg_type(module, field_ty);
                    (name, alg)
                })
                .collect();
            ReducerEntryData {
                reducer_name: reducer.name.to_string(),
                accessor_name,
                param_elem_data,
            }
        })
        .collect();

    let reducer_entries: Vec<SchemaReducerEntryRes> = reducer_data
        .iter()
        .map(|d| SchemaReducerEntryRes {
            reducer_name: &d.reducer_name,
            accessor_name: &d.accessor_name,
            param_elements: d
                .param_elem_data
                .iter()
                .map(|(name, alg)| SchemaNamedElementRes {
                    entry_name: name,
                    alg_type_expr: alg,
                })
                .collect(),
        })
        .collect();

    // ── Procedure entries ────────────────────────────────────────────
    let procedure_data: Vec<ProcedureEntryData> = iter_procedures(module, options.visibility)
        .map(|proc| {
            let param_elem_data: Vec<_> = proc
                .params_for_generate
                .elements
                .iter()
                .map(|(field, field_ty)| {
                    let name = field.deref().to_string();
                    let alg = render_schema_alg_type(module, field_ty);
                    (name, alg)
                })
                .collect();
            let return_type_expr = render_schema_alg_type(module, &proc.return_type_for_generate);
            ProcedureEntryData {
                procedure_name: proc.name.to_string(),
                accessor_name: proc.accessor_name.deref().to_case(Case::Camel),
                param_elem_data,
                return_type_expr,
            }
        })
        .collect();

    let procedure_entries: Vec<SchemaProcedureEntryRes> = procedure_data
        .iter()
        .map(|d| SchemaProcedureEntryRes {
            procedure_name: &d.procedure_name,
            accessor_name: &d.accessor_name,
            param_elements: d
                .param_elem_data
                .iter()
                .map(|(name, alg)| SchemaNamedElementRes {
                    entry_name: name,
                    alg_type_expr: alg,
                })
                .collect(),
            return_type_expr: &d.return_type_expr,
        })
        .collect();

    // ── Compose the file ─────────────────────────────────────────────
    OutputFile {
        filename: format!("{root_module}__Schema.res"),
        code: StdbSchemaRes {
            header: AutoGenHeaderRes,
            sdk_module: &sdk_module,
            cli_version: &cli_version,
            type_bindings: type_bindings_str.trim_end(),
            table_entries,
            reducer_entries,
            procedure_entries,
            all_table_names,
            subscribable_table_names,
        }
        .to_string(),
    }
}

// ---------------------------------------------------------------------------
// Intermediate data structs (own strings so template structs can borrow)
// ---------------------------------------------------------------------------

struct TableEntryData {
    accessor_name: String,
    source_name: String,
    is_event: bool,
    // Row product elements: (field_name, alg_type_expr)
    row_elem_data: Vec<(String, String)>,
    // Column entries: (col_name, is_primary_key, alg_type_expr)
    col_data: Vec<(String, bool, String)>,
    // Index entries: (index_name, pre-rendered columns string)
    index_data: Vec<(String, String)>,
    // Constraint entries: (constraint_name, pre-rendered columns string)
    constraint_data: Vec<(String, String)>,
}

/// Pre-render a list of column names as `"col1", "col2"`.
fn render_columns_str(cols: &[String]) -> String {
    cols.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ")
}

impl TableEntryData {
    fn to_template(&self) -> SchemaTableEntryRes<'_> {
        SchemaTableEntryRes {
            accessor_name: &self.accessor_name,
            source_name: &self.source_name,
            row_elements: self
                .row_elem_data
                .iter()
                .map(|(name, alg)| SchemaNamedElementRes {
                    entry_name: name,
                    alg_type_expr: alg,
                })
                .collect(),
            columns: self
                .col_data
                .iter()
                .map(|(name, is_pk, alg)| SchemaColumnEntryRes {
                    col_name: name,
                    is_primary_key: *is_pk,
                    alg_type_expr: alg,
                })
                .collect(),
            indexes: self
                .index_data
                .iter()
                .map(|(name, cols_str)| SchemaIndexEntryRes {
                    index_name: name,
                    columns_str: cols_str,
                })
                .collect(),
            constraints: self
                .constraint_data
                .iter()
                .map(|(name, cols_str)| SchemaConstraintEntryRes {
                    constraint_name: name,
                    columns_str: cols_str,
                })
                .collect(),
            is_event: self.is_event,
        }
    }
}

struct ReducerEntryData {
    reducer_name: String,
    accessor_name: String,
    param_elem_data: Vec<(String, String)>,
}

struct ProcedureEntryData {
    procedure_name: String,
    accessor_name: String,
    param_elem_data: Vec<(String, String)>,
    return_type_expr: String,
}

// ---------------------------------------------------------------------------
// Table/view entry builder (shared between tables and views)
// ---------------------------------------------------------------------------

/// Trait to unify `TableDef` and `ViewDef` for schema generation.
trait TableLike {
    fn accessor_name(&self) -> &Identifier;
    fn name(&self) -> &Identifier;
    fn product_type_ref(&self) -> spacetimedb_lib::sats::AlgebraicTypeRef;
    fn primary_key(&self) -> Option<spacetimedb_primitives::ColId>;
    fn is_event(&self) -> bool;
    fn indexes(&self) -> Vec<&spacetimedb_schema::def::IndexDef>;
    fn constraints(&self) -> Vec<&spacetimedb_schema::def::ConstraintDef>;
}

impl TableLike for &spacetimedb_schema::def::TableDef {
    fn accessor_name(&self) -> &Identifier {
        &(*self).accessor_name
    }
    fn name(&self) -> &Identifier {
        &(*self).name
    }
    fn product_type_ref(&self) -> spacetimedb_lib::sats::AlgebraicTypeRef {
        (*self).product_type_ref
    }
    fn primary_key(&self) -> Option<spacetimedb_primitives::ColId> {
        (*self).primary_key
    }
    fn is_event(&self) -> bool {
        (*self).is_event
    }
    fn indexes(&self) -> Vec<&spacetimedb_schema::def::IndexDef> {
        iter_indexes(*self).collect()
    }
    fn constraints(&self) -> Vec<&spacetimedb_schema::def::ConstraintDef> {
        iter_constraints(*self).collect()
    }
}

impl TableLike for &spacetimedb_schema::def::ViewDef {
    fn accessor_name(&self) -> &Identifier {
        &(*self).accessor_name
    }
    fn name(&self) -> &Identifier {
        &(*self).name
    }
    fn product_type_ref(&self) -> spacetimedb_lib::sats::AlgebraicTypeRef {
        (*self).product_type_ref
    }
    fn primary_key(&self) -> Option<spacetimedb_primitives::ColId> {
        None
    }
    fn is_event(&self) -> bool {
        false
    }
    fn indexes(&self) -> Vec<&spacetimedb_schema::def::IndexDef> {
        vec![]
    }
    fn constraints(&self) -> Vec<&spacetimedb_schema::def::ConstraintDef> {
        vec![]
    }
}

fn build_table_entry_data(module: &ModuleDef, table: impl TableLike) -> TableEntryData {
    let type_ref = table.product_type_ref();
    let product_def = module.typespace_for_generate()[type_ref].as_product().unwrap();
    let primary_key = table.primary_key();

    // Row elements — builds the productType for BSATN deserialization
    let row_elem_data: Vec<(String, String)> = product_def
        .elements
        .iter()
        .map(|(field, field_ty)| {
            let name = field.deref().to_string();
            let alg = render_schema_alg_type(module, field_ty);
            (name, alg)
        })
        .collect();

    // Column entries — builds the columns Dict for query builder
    let col_data: Vec<(String, bool, String)> = product_def
        .elements
        .iter()
        .enumerate()
        .map(|(i, (field, field_ty))| {
            let col_name = field.deref().to_case(Case::Camel);
            let is_pk = primary_key.is_some_and(|pk| pk.idx() == i);
            let alg = render_schema_alg_type(module, field_ty);
            (col_name, is_pk, alg)
        })
        .collect();

    // Index entries (pre-render columns as quoted strings)
    let index_data: Vec<(String, String)> = table
        .indexes()
        .into_iter()
        .filter(|idx| !idx.generated())
        .map(|idx| {
            let idx_name = idx
                .accessor_name
                .as_ref()
                .map_or_else(|| idx.name.deref().to_string(), |n| n.deref().to_string());
            let cols: Vec<String> = idx
                .algorithm
                .columns()
                .iter()
                .map(|col_id| {
                    let (field_name, _) = &product_def.elements[col_id.idx()];
                    field_name.deref().to_case(Case::Camel)
                })
                .collect();
            (idx_name, render_columns_str(&cols))
        })
        .collect();

    // Constraint entries (pre-render columns as quoted strings)
    let constraint_data: Vec<(String, String)> = table
        .constraints()
        .into_iter()
        .map(|c| {
            let cols: Vec<String> = c
                .data
                .unique_columns()
                .into_iter()
                .flat_map(|cs| cs.iter())
                .map(|col_id| {
                    let (field_name, _) = &product_def.elements[col_id.idx()];
                    field_name.deref().to_case(Case::Camel)
                })
                .collect();
            (c.name.to_string(), render_columns_str(&cols))
        })
        .collect();

    TableEntryData {
        accessor_name: table.accessor_name().deref().to_string(),
        source_name: table.name().deref().to_string(),
        is_event: table.is_event(),
        row_elem_data,
        col_data,
        index_data,
        constraint_data,
    }
}
