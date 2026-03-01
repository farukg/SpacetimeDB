//! `StdbDisplay.res` generator — mechanical unwrappers and toString functions.
//!
//! Emits three sections:
//!
//! 1. **SDK type helpers** — fixed bindings for `identity`, `connectionId`,
//!    `timestamp`, and `amount` (always emitted, no schema data needed).
//! 2. **Newtype unwrappers** — one `let` per Product type with exactly 1 field.
//! 3. **Enum toString** — one `switch` function per PlainEnum type.

use super::helpers::{rescript_constructor_name, rescript_field_name, rescript_module_name};
use super::templates::{
    AutoGenHeaderRes, DisplayEnumArmRes, DisplayEnumToStringRes, DisplayUnwrapperRes, StdbDisplayRes,
};
use crate::util::{iter_types, type_ref_name};
use crate::OutputFile;

use convert_case::{Case, Casing};
use spacetimedb_schema::def::ModuleDef;
use spacetimedb_schema::type_for_generate::AlgebraicTypeDef;
use std::ops::Deref;

pub(super) fn generate_display_file(module: &ModuleDef) -> OutputFile {
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
    for data in &enum_data {
        let arms: Vec<DisplayEnumArmRes> = data
            .constructors
            .iter()
            .map(|c| DisplayEnumArmRes { constructor: c })
            .collect();
        let tmpl = DisplayEnumToStringRes {
            fn_name: &data.fn_name,
            module_name: &data.module_name,
            arms,
        };
        enum_to_strings.push_str(&tmpl.to_string());
    }

    let display = StdbDisplayRes {
        header: AutoGenHeaderRes,
        unwrappers: &unwrappers,
        enum_to_strings: &enum_to_strings,
    };

    OutputFile {
        filename: "StdbDisplay.res".to_string(),
        code: display.to_string(),
    }
}
