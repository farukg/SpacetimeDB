//! Display projection generator — `type display` + `let toDisplay` per table file.
//!
//! Analyzes each field of a table's row type and emits:
//! - A `display` record type with primitives unwrapped for UI consumption
//! - A `toDisplay: t => display` function that performs the conversion
//!
//! Field transformation rules:
//! - **Newtype (single-field product Ref)**: `field: string` (toKey) + `fieldRaw: innerType` (value)
//! - **PlainEnum Ref**: `field: Enum.t` (preserved) + `fieldLabel: string` (toString)
//! - **bigint**: `field: float` (BigInt.toFloat)
//! - **Timestamp**: `field: float` (Display.timestamp)
//! - **Identity**: `field: string` (Display.identity)
//! - **ConnectionId**: `field: string` (Display.connectionId)
//! - **Option<T>**: recurse into T, wrap conversion in Option.map
//! - **Everything else**: pass through unchanged

use super::helpers::{rescript_field_name, rescript_module_name, TypeRefStyle};
use super::templates::{DisplayProjectionFieldRes, DisplayProjectionRes, DisplayProjectionTypeFieldRes};
use crate::util::type_ref_name;

use convert_case::{Case, Casing};
use spacetimedb_lib::sats::layout::PrimitiveType;
use spacetimedb_schema::def::ModuleDef;
use spacetimedb_schema::identifier::Identifier;
use spacetimedb_schema::type_for_generate::{AlgebraicTypeDef, AlgebraicTypeUse};
use std::ops::Deref;

/// Classification of a field for display projection purposes.
enum FieldProjection {
    /// Newtype: emit two fields (toKey + raw value).
    Newtype {
        /// Display type for the key field (usually `string`).
        key_type: String,
        /// Display type for the raw value field.
        raw_type: String,
        /// Expression to convert `row.field` to key (e.g., `row.field->Module.toKey`).
        to_key_expr: String,
        /// Expression to get raw value (e.g., `row.field->Module.value`).
        to_raw_expr: String,
    },
    /// PlainEnum: keep typed field + add label field.
    PlainEnum {
        /// The original enum type (e.g., `Stdb__Types.NamespaceTestC.t`).
        enum_type: String,
        /// Expression for toString (e.g., `row.field->Stdb__Display.enumToString`).
        to_label_expr: String,
    },
    /// Bigint → float.
    BigintToFloat { convert_expr: String },
    /// Timestamp → float.
    TimestampToFloat { convert_expr: String },
    /// Identity → string.
    IdentityToString { convert_expr: String },
    /// ConnectionId → string.
    ConnectionIdToString { convert_expr: String },
    /// Option wrapping an inner projection that needs conversion.
    OptionWrapped {
        /// The display type of the inner value.
        inner_display_type: String,
        /// Expression mapping the option (e.g., `row.field->Option.map(v => v->BigInt.toFloat)`).
        convert_expr: String,
    },
    /// Pass-through: same type, no conversion.
    Passthrough { type_str: String },
}

/// Owned data for a display field before borrowing for templates.
struct DisplayFieldData {
    camel_name: String,
    type_str: String,
    convert_expr: String,
}

/// Render the display projection section (type + toDisplay) for a table file.
///
/// Returns empty string if the table has zero fields (unit type).
pub(super) fn render_display_section(
    module: &ModuleDef,
    elements: &[(Identifier, AlgebraicTypeUse)],
    root_module: &str,
) -> String {
    if elements.is_empty() {
        return String::new();
    }

    let typespace = module.typespace_for_generate();
    // Bare module name — the calling file has `module Display = {root_module}__Display` via sibling_opens.
    let display_module = "Display";

    let mut display_fields: Vec<DisplayFieldData> = Vec::new();

    for (field_ident, field_ty) in elements {
        let raw_name = field_ident.deref();
        let camel = rescript_field_name(raw_name.to_case(Case::Camel));
        let row_access = format!("row.{camel}");

        let projection = classify_field(module, &typespace, field_ty, &row_access, root_module, &display_module);

        match projection {
            FieldProjection::Newtype {
                key_type,
                raw_type,
                to_key_expr,
                to_raw_expr,
            } => {
                display_fields.push(DisplayFieldData {
                    camel_name: camel.clone(),
                    type_str: key_type,
                    convert_expr: to_key_expr,
                });
                display_fields.push(DisplayFieldData {
                    camel_name: format!("{camel}Raw"),
                    type_str: raw_type,
                    convert_expr: to_raw_expr,
                });
            }
            FieldProjection::PlainEnum {
                enum_type,
                to_label_expr,
            } => {
                display_fields.push(DisplayFieldData {
                    camel_name: camel.clone(),
                    type_str: enum_type,
                    convert_expr: row_access.clone(),
                });
                display_fields.push(DisplayFieldData {
                    camel_name: format!("{camel}Label"),
                    type_str: "string".to_string(),
                    convert_expr: to_label_expr,
                });
            }
            FieldProjection::BigintToFloat { convert_expr } => {
                display_fields.push(DisplayFieldData {
                    camel_name: camel,
                    type_str: "float".to_string(),
                    convert_expr,
                });
            }
            FieldProjection::TimestampToFloat { convert_expr } => {
                display_fields.push(DisplayFieldData {
                    camel_name: camel,
                    type_str: "float".to_string(),
                    convert_expr,
                });
            }
            FieldProjection::IdentityToString { convert_expr } => {
                display_fields.push(DisplayFieldData {
                    camel_name: camel,
                    type_str: "string".to_string(),
                    convert_expr,
                });
            }
            FieldProjection::ConnectionIdToString { convert_expr } => {
                display_fields.push(DisplayFieldData {
                    camel_name: camel,
                    type_str: "string".to_string(),
                    convert_expr,
                });
            }
            FieldProjection::OptionWrapped {
                inner_display_type,
                convert_expr,
            } => {
                display_fields.push(DisplayFieldData {
                    camel_name: camel,
                    type_str: format!("option<{inner_display_type}>"),
                    convert_expr,
                });
            }
            FieldProjection::Passthrough { type_str } => {
                display_fields.push(DisplayFieldData {
                    camel_name: camel,
                    type_str,
                    convert_expr: row_access,
                });
            }
        }
    }

    // Build template structs from owned data.
    let type_fields: Vec<DisplayProjectionTypeFieldRes> = display_fields
        .iter()
        .map(|f| DisplayProjectionTypeFieldRes {
            camel_name: &f.camel_name,
            type_str: &f.type_str,
        })
        .collect();

    let body_fields: Vec<DisplayProjectionFieldRes> = display_fields
        .iter()
        .map(|f| DisplayProjectionFieldRes {
            camel_name: &f.camel_name,
            convert_expr: &f.convert_expr,
        })
        .collect();

    DisplayProjectionRes {
        type_fields,
        body_fields,
    }
    .to_string()
}

/// Classify a single field for display projection.
fn classify_field(
    module: &ModuleDef,
    typespace: &spacetimedb_schema::type_for_generate::TypespaceForGenerate,
    ty: &AlgebraicTypeUse,
    row_access: &str,
    root_module: &str,
    display_module: &str,
) -> FieldProjection {
    match ty {
        // Bigint primitives → float
        AlgebraicTypeUse::Primitive(prim) => match prim {
            PrimitiveType::I64
            | PrimitiveType::U64
            | PrimitiveType::I128
            | PrimitiveType::U128
            | PrimitiveType::I256
            | PrimitiveType::U256 => FieldProjection::BigintToFloat {
                convert_expr: format!("{row_access}->BigInt.toFloat"),
            },
            _ => FieldProjection::Passthrough {
                type_str: super::helpers::render_res_type(module, ty, TypeRefStyle::ViaGateway, root_module),
            },
        },

        // Timestamp → float
        AlgebraicTypeUse::Timestamp => FieldProjection::TimestampToFloat {
            convert_expr: format!("{row_access}->{display_module}.timestamp"),
        },

        // Identity → string
        AlgebraicTypeUse::Identity => FieldProjection::IdentityToString {
            convert_expr: format!("{row_access}->{display_module}.identity"),
        },

        // ConnectionId → string
        AlgebraicTypeUse::ConnectionId => FieldProjection::ConnectionIdToString {
            convert_expr: format!("{row_access}->{display_module}.connectionId"),
        },

        // Option<T> → recurse into T
        AlgebraicTypeUse::Option(inner) => {
            let inner_proj = classify_field(module, typespace, inner, "v", root_module, display_module);
            match inner_proj {
                FieldProjection::Passthrough { type_str } => FieldProjection::Passthrough {
                    type_str: format!("option<{type_str}>"),
                },
                FieldProjection::BigintToFloat { convert_expr }
                | FieldProjection::TimestampToFloat { convert_expr } => FieldProjection::OptionWrapped {
                    inner_display_type: "float".to_string(),
                    convert_expr: format!("{row_access}->Option.map(v => {convert_expr})"),
                },
                FieldProjection::IdentityToString { convert_expr }
                | FieldProjection::ConnectionIdToString { convert_expr } => FieldProjection::OptionWrapped {
                    inner_display_type: "string".to_string(),
                    convert_expr: format!("{row_access}->Option.map(v => {convert_expr})"),
                },
                FieldProjection::Newtype {
                    key_type, to_key_expr, ..
                } => FieldProjection::OptionWrapped {
                    inner_display_type: key_type,
                    convert_expr: format!("{row_access}->Option.map(v => {to_key_expr})"),
                },
                FieldProjection::PlainEnum { enum_type, .. } => FieldProjection::Passthrough {
                    type_str: format!("option<{enum_type}>"),
                },
                _ => FieldProjection::Passthrough {
                    type_str: super::helpers::render_res_type(module, ty, TypeRefStyle::ViaGateway, root_module),
                },
            }
        }

        // Ref to named type → check if newtype or plain enum
        AlgebraicTypeUse::Ref(reference) => {
            let pascal_name = type_ref_name(module, *reference);
            let module_name = rescript_module_name(&pascal_name);
            // Bare module name — the calling file has `module Types = {root_module}__Types` via sibling_opens.
            let types_module = "Types";

            match &typespace[*reference] {
                AlgebraicTypeDef::Product(product) if product.elements.len() == 1 => {
                    // Single-field product = newtype
                    let (_inner_field, inner_ty) = &product.elements[0];
                    let inner_type_str =
                        super::helpers::render_res_type(module, inner_ty, TypeRefStyle::ViaGateway, root_module);

                    // Check if toKey exists for this newtype
                    let has_to_key = super::helpers::render_to_key_expr(inner_ty, "unused").is_some();

                    let key_type = if has_to_key {
                        "string".to_string()
                    } else {
                        inner_type_str.clone()
                    };
                    let to_key_expr = if has_to_key {
                        format!("{row_access}->{types_module}.{module_name}.toKey")
                    } else {
                        format!("{row_access}->{types_module}.{module_name}.value")
                    };

                    FieldProjection::Newtype {
                        key_type,
                        raw_type: inner_type_str,
                        to_key_expr,
                        to_raw_expr: format!("{row_access}->{types_module}.{module_name}.value"),
                    }
                }
                AlgebraicTypeDef::PlainEnum(_) => {
                    let fn_name = format!("{}ToString", pascal_name.to_case(Case::Camel));
                    FieldProjection::PlainEnum {
                        enum_type: format!("{types_module}.{module_name}.t"),
                        to_label_expr: format!("{row_access}->{display_module}.{fn_name}"),
                    }
                }
                _ => {
                    // Multi-field product or tagged sum → pass through
                    FieldProjection::Passthrough {
                        type_str: super::helpers::render_res_type(module, ty, TypeRefStyle::ViaGateway, root_module),
                    }
                }
            }
        }

        // Everything else → pass through
        _ => FieldProjection::Passthrough {
            type_str: super::helpers::render_res_type(module, ty, TypeRefStyle::ViaGateway, root_module),
        },
    }
}
