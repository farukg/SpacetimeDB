//! `Stdb__Labels.res` generator — diff-based label/translation stubs.
//!
//! Generates one switch function per `PlainEnum` type. On first run, all
//! strings are `"TODO"`. On subsequent runs with `existing_content`, existing
//! human-written strings are preserved and only new variants get `"TODO"`.

use super::helpers::{rescript_constructor_name, rescript_module_name};
use crate::util::{iter_types, type_ref_name};
use crate::OutputFile;

use convert_case::{Case, Casing};
use spacetimedb_schema::def::ModuleDef;
use spacetimedb_schema::type_for_generate::AlgebraicTypeDef;
use std::collections::HashMap;
use std::fmt::Write;
use std::ops::Deref;

/// Parse an existing Labels file to extract `fn_name → (variant → string)` mappings.
fn parse_existing(content: &str) -> HashMap<String, HashMap<String, String>> {
    let mut result: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_fn: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect function header: `let fooLabel = (v: Module.t): string =>`
        if let Some(rest) = trimmed.strip_prefix("let ") {
            if rest.contains("): string =>") || rest.contains("): string=>") {
                if let Some(name_end) = rest.find(' ') {
                    let fn_name = rest[..name_end].to_string();
                    result.entry(fn_name.clone()).or_default();
                    current_fn = Some(fn_name);
                }
            }
            continue;
        }

        // Detect switch arm: `| VariantName => "string value"` or `| VariantName(_) => "string value"`
        if let Some(rest) = trimmed.strip_prefix("| ") {
            if let Some(fn_name) = &current_fn {
                if let Some(arrow_pos) = rest.find(" => \"") {
                    let variant_raw = rest[..arrow_pos].trim();
                    // Strip wildcard payload pattern: `Constructor(_)` → `Constructor`
                    let variant = if let Some(stripped) = variant_raw.strip_suffix("(_)") {
                        stripped.to_string()
                    } else {
                        variant_raw.to_string()
                    };
                    let after_arrow = &rest[arrow_pos + 5..]; // skip ` => "`
                    if let Some(end_quote) = after_arrow.rfind('"') {
                        let value = after_arrow[..end_quote].to_string();
                        result.get_mut(fn_name).unwrap().insert(variant, value);
                    }
                }
            }
            continue;
        }

        // Detect end of switch (closing brace resets current function).
        if trimmed == "}" {
            current_fn = None;
        }
    }

    result
}

/// Intermediate data for a PlainEnum or Sum type.
struct LabelEnumData {
    fn_name: String,
    module_name: String,
    constructors: Vec<String>,
    /// For Sum types: which constructors carry payloads (for wildcard pattern generation).
    /// For PlainEnum: all `false`.
    payload_constructors: Vec<bool>,
}

/// Generate `{root}__Labels.res` with optional diff-based merge.
///
/// When `existing_content` is `None`, generates fresh stubs with `"TODO"`.
/// When `Some`, parses existing content to preserve human-written strings.
pub fn generate_labels_file(module: &ModuleDef, root_module: &str, existing_content: Option<&str>) -> OutputFile {
    let types: Vec<_> = iter_types(module).collect();
    let typespace = module.typespace_for_generate();

    // Collect PlainEnum and Sum data for label generation.
    let enum_data: Vec<LabelEnumData> = types
        .iter()
        .filter_map(|typ| match &typespace[typ.ty] {
            AlgebraicTypeDef::PlainEnum(plain_enum) => {
                let pascal = type_ref_name(module, typ.ty);
                let module_name = rescript_module_name(&pascal);
                let fn_name = format!("{}Label", pascal.to_case(Case::Camel));
                let constructors: Vec<String> = plain_enum
                    .variants
                    .iter()
                    .map(|v| rescript_constructor_name(v.deref()))
                    .collect();
                let payload_constructors = vec![false; constructors.len()];
                Some(LabelEnumData {
                    fn_name,
                    module_name,
                    constructors,
                    payload_constructors,
                })
            }
            AlgebraicTypeDef::Sum(sum) => {
                let pascal = type_ref_name(module, typ.ty);
                let module_name = rescript_module_name(&pascal);
                let fn_name = format!("{}Label", pascal.to_case(Case::Camel));
                let mut constructors = Vec::new();
                let mut payload_constructors = Vec::new();
                for (name, variant_ty) in sum.variants.iter() {
                    constructors.push(rescript_constructor_name(name.deref()));
                    let is_payload = !matches!(
                        variant_ty,
                        spacetimedb_schema::type_for_generate::AlgebraicTypeUse::Unit
                    );
                    payload_constructors.push(is_payload);
                }
                Some(LabelEnumData {
                    fn_name,
                    module_name,
                    constructors,
                    payload_constructors,
                })
            }
            _ => None,
        })
        .collect();

    if enum_data.is_empty() {
        return OutputFile {
            filename: format!("{root_module}__Labels.res"),
            code: String::new(),
        };
    }

    // Parse existing content for merge.
    let existing = existing_content.map(parse_existing).unwrap_or_default();

    // Build output.
    let mut code = String::new();
    writeln!(
        code,
        "// THIS FILE IS AUTOMATICALLY GENERATED BY SPACETIMEDB.\n\
         //\n\
         // EXCEPTION: Translation strings are human-maintained.\n\
         // New enum variants get \"TODO\" — fill them in.\n\
         // Existing strings are preserved across regeneration.\n\
         \n\
         open {root_module}__Types\n\
         \n\
         module De = {{"
    )
    .unwrap();

    for (i, data) in enum_data.iter().enumerate() {
        if i > 0 {
            writeln!(code).unwrap();
        }
        let existing_map = existing.get(&data.fn_name);

        writeln!(
            code,
            "  let {} = (v: {}.t): string =>\n    switch v {{",
            data.fn_name, data.module_name
        )
        .unwrap();

        for (i, constructor) in data.constructors.iter().enumerate() {
            let value = existing_map
                .and_then(|m| m.get(constructor.as_str()))
                .map(|s| s.as_str())
                .unwrap_or("TODO");
            let pattern = if data.payload_constructors[i] {
                format!("{constructor}(_)")
            } else {
                constructor.to_string()
            };
            writeln!(code, "    | {pattern} => \"{value}\"").unwrap();
        }

        writeln!(code, "    }}").unwrap();
    }

    writeln!(code, "}}").unwrap();

    OutputFile {
        filename: format!("{root_module}__Labels.res"),
        code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_preserves_existing_strings() {
        let content = r#"
open Stdb__Types

module De = {
  let statusLabel = (v: Status.t): string =>
    switch v {
    | Active => "Aktiv"
    | Inactive => "Inaktiv"
    }
}
"#;
        let parsed = parse_existing(content);
        let status = parsed.get("statusLabel").unwrap();
        assert_eq!(status.get("Active").unwrap(), "Aktiv");
        assert_eq!(status.get("Inactive").unwrap(), "Inaktiv");
    }

    #[test]
    fn parse_handles_empty() {
        let parsed = parse_existing("");
        assert!(parsed.is_empty());
    }
}
