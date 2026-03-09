//! `StdbDisplay.res` generator — mechanical unwrappers and toString functions.
//!
//! Emits three sections:
//!
//! 1. **SDK type helpers** — fixed bindings for `identity`, `connectionId`,
//!    `timestamp`, and `amount` (always emitted, no schema data needed).
//! 2. **Newtype unwrappers** — one `let` per Product type with exactly 1 field.
//! 3. **Enum toString** — one `switch` function per PlainEnum type.

use super::helpers::{rescript_constructor_name, rescript_field_name, rescript_module_name, sibling_opens};
use super::templates::{
    AutoGenHeaderRes, DisplayEnumArmRes, DisplayEnumFromStringArmRes, DisplaySumPayloadArmRes, DisplaySumUnitArmRes,
    DisplayUnwrapperRes, StdbDisplayRes, SwitchFunctionRes,
};
use crate::util::{iter_types, type_ref_name};
use crate::OutputFile;

use convert_case::{Case, Casing};
use spacetimedb_schema::def::ModuleDef;
use spacetimedb_schema::type_for_generate::{AlgebraicTypeDef, AlgebraicTypeUse};
use std::ops::Deref;

pub(super) fn generate_display_file(module: &ModuleDef, root_module: &str) -> OutputFile {
    let types: Vec<_> = iter_types(module).collect();
    let typespace = module.typespace_for_generate();

    // ── Newtype unwrappers ───────────────────────────────────────────────────
    //
    // Build owned data first, then borrow for template structs.
    struct UnwrapperData {
        fn_name: String,
        module_name: String,
        field_name: String,
    }

    let unwrapper_data: Vec<UnwrapperData> = types
        .iter()
        .filter_map(|typ| {
            if let AlgebraicTypeDef::Product(product) = &typespace[typ.ty] {
                if product.elements.len() == 1 {
                    let pascal = type_ref_name(module, typ.ty);
                    let module_name = rescript_module_name(&pascal);
                    let fn_name = pascal.to_case(Case::Camel);
                    let (field_ident, _field_ty) = &product.elements[0];
                    let field_name = rescript_field_name(field_ident.deref().to_case(Case::Camel));
                    return Some(UnwrapperData {
                        fn_name,
                        module_name,
                        field_name,
                    });
                }
            }
            None
        })
        .collect();

    let mut unwrappers = String::new();
    for data in &unwrapper_data {
        let tmpl = DisplayUnwrapperRes {
            fn_name: &data.fn_name,
            module_name: &data.module_name,
            field_name: &data.field_name,
        };
        unwrappers.push_str(&tmpl.to_string());
    }

    // ── Enum toString functions ──────────────────────────────────────────────
    //
    // Build owned constructor names first, then borrow for template structs.
    struct EnumData {
        fn_name: String,
        module_name: String,
        constructors: Vec<String>,
    }

    let enum_data: Vec<EnumData> = types
        .iter()
        .filter_map(|typ| {
            if let AlgebraicTypeDef::PlainEnum(plain_enum) = &typespace[typ.ty] {
                let pascal = type_ref_name(module, typ.ty);
                let module_name = rescript_module_name(&pascal);
                let fn_name = format!("{}ToString", pascal.to_case(Case::Camel));
                let constructors: Vec<String> = plain_enum
                    .variants
                    .iter()
                    .map(|v| rescript_constructor_name(v.deref()))
                    .collect();
                return Some(EnumData {
                    fn_name,
                    module_name,
                    constructors,
                });
            }
            None
        })
        .collect();

    let mut enum_to_strings = String::new();
    let mut enum_from_strings = String::new();
    for data in &enum_data {
        let to_arm_strings: Vec<String> = data
            .constructors
            .iter()
            .map(|c| DisplayEnumArmRes {
                module_name: &data.module_name,
                constructor: c,
            })
            .map(|arm| arm.to_string())
            .collect();
        let to_arm_refs: Vec<&str> = to_arm_strings.iter().map(|s| s.as_str()).collect();
        enum_to_strings.push_str(
            &SwitchFunctionRes {
                fn_name: &data.fn_name,
                input_type: &format!("Types.{}.t", data.module_name),
                output_type: "string",
                arms: to_arm_refs,
                has_fallback: false,
                fallback_arm: "",
            }
            .to_string(),
        );

        let from_fn_name = data.fn_name.replace("ToString", "FromString");
        let from_arm_strings: Vec<String> = data
            .constructors
            .iter()
            .map(|c| DisplayEnumFromStringArmRes {
                module_name: &data.module_name,
                constructor: c,
            })
            .map(|arm| arm.to_string())
            .collect();
        let from_arm_refs: Vec<&str> = from_arm_strings.iter().map(|s| s.as_str()).collect();
        enum_from_strings.push_str(
            &SwitchFunctionRes {
                fn_name: &from_fn_name,
                input_type: "string",
                output_type: &format!("option<Types.{}.t>", data.module_name),
                arms: from_arm_refs,
                has_fallback: true,
                fallback_arm: "  | _ => None",
            }
            .to_string(),
        );
    }

    // ── Sum enum toString functions ─────────────────────────────────────
    //
    // Sum types (payload-carrying enums). Unit variants get simple string arms,
    // payload variants project the payload via its own toString or generic conversion.
    // No fromString is generated for Sum types (AD-037: ambiguous reconstruction).

    struct SumVariantData {
        constructor: String,
        /// None = unit variant, Some(payload_type) = payload variant
        payload_type: Option<AlgebraicTypeUse>,
    }

    struct SumEnumData {
        fn_name: String,
        module_name: String,
        variants: Vec<SumVariantData>,
    }

    let sum_data: Vec<SumEnumData> = types
        .iter()
        .filter_map(|typ| {
            if let AlgebraicTypeDef::Sum(sum) = &typespace[typ.ty] {
                let pascal = type_ref_name(module, typ.ty);
                let module_name = rescript_module_name(&pascal);
                let fn_name = format!("{}ToString", pascal.to_case(Case::Camel));
                let variants: Vec<SumVariantData> = sum
                    .variants
                    .iter()
                    .map(|(name, variant_ty)| {
                        let constructor = rescript_constructor_name(name.deref());
                        let payload_type = if matches!(variant_ty, AlgebraicTypeUse::Unit) {
                            None
                        } else {
                            Some(variant_ty.clone())
                        };
                        SumVariantData {
                            constructor,
                            payload_type,
                        }
                    })
                    .collect();
                return Some(SumEnumData {
                    fn_name,
                    module_name,
                    variants,
                });
            }
            None
        })
        .collect();

    let mut sum_to_strings = String::new();
    for data in &sum_data {
        // Pre-render each arm as a string since they're heterogeneous (unit vs payload).
        let arm_strings: Vec<String> = data
            .variants
            .iter()
            .map(|v| match &v.payload_type {
                None => DisplaySumUnitArmRes {
                    module_name: &data.module_name,
                    constructor: &v.constructor,
                }
                .to_string(),
                Some(payload_ty) => {
                    let payload_expr = render_payload_to_string(module, &typespace, payload_ty);
                    DisplaySumPayloadArmRes {
                        module_name: &data.module_name,
                        constructor: &v.constructor,
                        payload_expr: &payload_expr,
                    }
                    .to_string()
                }
            })
            .collect();
        let arm_refs: Vec<&str> = arm_strings.iter().map(|s| s.as_str()).collect();
        sum_to_strings.push_str(
            &SwitchFunctionRes {
                fn_name: &data.fn_name,
                input_type: &format!("Types.{}.t", data.module_name),
                output_type: "string",
                arms: arm_refs,
                has_fallback: false,
                fallback_arm: "",
            }
            .to_string(),
        );
    }

    let opens = sibling_opens(root_module, &["Types", "Sdk"]);
    let display = StdbDisplayRes {
        header: AutoGenHeaderRes,
        unwrappers: &unwrappers,
        enum_to_strings: &enum_to_strings,
        enum_from_strings: &enum_from_strings,
        sum_to_strings: &sum_to_strings,
        sibling_opens: &opens,
    };

    OutputFile {
        filename: format!("{root_module}__Display.res"),
        code: display.to_string(),
    }
}

/// Determine how to convert a Sum variant's payload to a string representation.
///
/// Returns a ReScript expression that converts the bound `payload` variable to string.
fn render_payload_to_string(
    module: &ModuleDef,
    typespace: &spacetimedb_schema::type_for_generate::TypespaceForGenerate,
    payload_ty: &AlgebraicTypeUse,
) -> String {
    match payload_ty {
        AlgebraicTypeUse::String => "payload".to_string(),
        AlgebraicTypeUse::Primitive(prim) => {
            use spacetimedb_lib::sats::layout::PrimitiveType;
            match prim {
                PrimitiveType::Bool => "string_of_bool(payload)".to_string(),
                PrimitiveType::I8
                | PrimitiveType::U8
                | PrimitiveType::I16
                | PrimitiveType::U16
                | PrimitiveType::I32
                | PrimitiveType::U32 => "Int.toString(payload)".to_string(),
                PrimitiveType::I64
                | PrimitiveType::U64
                | PrimitiveType::I128
                | PrimitiveType::U128
                | PrimitiveType::I256
                | PrimitiveType::U256 => "BigInt.toString(payload)".to_string(),
                PrimitiveType::F32 | PrimitiveType::F64 => "Float.toString(payload)".to_string(),
            }
        }
        AlgebraicTypeUse::Ref(reference) => {
            let pascal_name = type_ref_name(module, *reference);
            match &typespace[*reference] {
                // Payload is another PlainEnum → use its toString
                AlgebraicTypeDef::PlainEnum(_) => {
                    let fn_name = format!("{}ToString", pascal_name.to_case(Case::Camel));
                    format!("{fn_name}(payload)")
                }
                // Payload is another Sum → use its toString (recursive)
                AlgebraicTypeDef::Sum(_) => {
                    let fn_name = format!("{}ToString", pascal_name.to_case(Case::Camel));
                    format!("{fn_name}(payload)")
                }
                // Payload is a Product (struct) → use JSON.stringifyAny as fallback
                AlgebraicTypeDef::Product(_) => r#"JSON.stringifyAny(payload)->Option.getOr("<opaque>")"#.to_string(),
            }
        }
        // Fallback for other types
        _ => r#"JSON.stringifyAny(payload)->Option.getOr("<opaque>")"#.to_string(),
    }
}
