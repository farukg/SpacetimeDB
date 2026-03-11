//! `stdb-codegen.toml` configuration reader for ReScript codegen.
//!
//! Reads optional configuration from a TOML file placed next to the output
//! directory or project root. All fields have sensible defaults — the file
//! itself is optional.
//!
//! ## Example `stdb-codegen.toml`
//!
//! ```toml
//! root_module = "Stdb"
//! call_mode = "observer"          # "hooks" | "observer" | "combined"
//! field_naming = "camelCase"      # "camelCase" | "snake_case"
//! output_dir_strategy = "subdirectories"  # "flat" | "subdirectories"
//! table_style = "inline"          # "inline" | "functor"
//! ```

use crate::CallMode;
use serde::Deserialize;
use std::path::Path;

/// Output directory strategy for generated files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum OutputDirStrategy {
    #[serde(rename = "flat")]
    Flat,
    #[default]
    #[serde(rename = "subdirectories")]
    Subdirectories,
}

/// Table file generation style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum TableStyle {
    /// Current behavior: all boilerplate inlined per table file.
    #[default]
    #[serde(rename = "inline")]
    Inline,
    /// Generic functor: shared `Stdb__TableFunctor.res` + thin per-table config modules.
    #[serde(rename = "functor")]
    Functor,
}

/// ReScript codegen configuration, deserialized from `stdb-codegen.toml`.
///
/// All fields are optional with defaults matching the current behavior.
/// Unknown keys are silently ignored (forward-compatible).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RescriptCodegenConfig {
    /// Root module prefix for generated files. Default: `"Stdb"`.
    /// Becomes the file prefix: `Stdb__Types.res`, `Stdb__Schema.res`, etc.
    pub root_module: String,

    /// Controls what call surface is emitted.
    /// - `"hooks"`: SDK-native React hooks only.
    /// - `"observer"`: Module functor API only, no React hooks.
    /// - `"combined"` (default): Both hook and observer surfaces.
    #[serde(deserialize_with = "deserialize_call_mode")]
    pub call_mode: CallMode,

    /// Record field naming strategy.
    /// - `"camelCase"` (default): fields use camelCase + `@as("snake_case")`.
    /// - `"snake_case"`: fields use snake_case identifiers, no `@as`.
    pub field_naming: FieldNaming,

    /// Output directory strategy.
    /// - `"flat"`: all files in output_dir/.
    /// - `"subdirectories"` (default): files grouped by namespace level.
    pub output_dir_strategy: OutputDirStrategy,

    /// Table file generation style.
    /// - `"inline"` (default): all boilerplate inlined per table file.
    /// - `"functor"`: generic `Stdb__TableFunctor.res` + thin per-table config modules.
    pub table_style: TableStyle,
}

impl Default for RescriptCodegenConfig {
    fn default() -> Self {
        Self {
            root_module: "Stdb".to_string(),
            call_mode: CallMode::Combined,
            field_naming: FieldNaming::CamelCase,
            output_dir_strategy: OutputDirStrategy::Subdirectories,
            table_style: TableStyle::Inline,
        }
    }
}

/// Field naming strategy for generated ReScript record types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum FieldNaming {
    #[default]
    #[serde(rename = "camelCase")]
    CamelCase,
    #[serde(rename = "snake_case")]
    SnakeCase,
}

fn deserialize_call_mode<'de, D>(deserializer: D) -> Result<CallMode, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
        "hooks" => Ok(CallMode::Hooks),
        "observer" => Ok(CallMode::Observer),
        "combined" => Ok(CallMode::Combined),
        other => Err(serde::de::Error::custom(format!(
            "unknown call_mode '{other}', expected 'hooks', 'observer', or 'combined'"
        ))),
    }
}

/// The config file name to look for.
pub const CONFIG_FILENAME: &str = "stdb-codegen.toml";

/// Load ReScript codegen config by searching for `stdb-codegen.toml` in the
/// given search paths (tried in order). Returns defaults if no file is found.
///
/// # Arguments
/// * `search_paths` — directories to search for `stdb-codegen.toml`, in priority order.
///   Typically: `[out_dir, config_dir]` or `[out_dir]`.
pub fn load_config(search_paths: &[&Path]) -> Result<RescriptCodegenConfig, anyhow::Error> {
    for dir in search_paths {
        let config_path = dir.join(CONFIG_FILENAME);
        if config_path.is_file() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: RescriptCodegenConfig = toml::from_str(&content)?;
            return Ok(config);
        }
    }
    Ok(RescriptCodegenConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RescriptCodegenConfig::default();
        assert_eq!(config.root_module, "Stdb");
        assert_eq!(config.call_mode, CallMode::Combined);
        assert_eq!(config.field_naming, FieldNaming::CamelCase);
    }

    #[test]
    fn test_parse_minimal_toml() {
        let toml_str = "";
        let config: RescriptCodegenConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.root_module, "Stdb");
        assert_eq!(config.call_mode, CallMode::Combined);
    }

    #[test]
    fn test_parse_full_toml() {
        let toml_str = r#"
root_module = "App"
call_mode = "observer"
field_naming = "snake_case"
"#;
        let config: RescriptCodegenConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.root_module, "App");
        assert_eq!(config.call_mode, CallMode::Observer);
        assert_eq!(config.field_naming, FieldNaming::SnakeCase);
    }

    #[test]
    fn test_parse_partial_toml() {
        let toml_str = r#"root_module = "MyDb""#;
        let config: RescriptCodegenConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.root_module, "MyDb");
        assert_eq!(config.call_mode, CallMode::Combined); // default
        assert_eq!(config.field_naming, FieldNaming::CamelCase); // default
    }

    #[test]
    fn test_unknown_keys_ignored() {
        let toml_str = r#"
root_module = "Stdb"
future_option = true
another_thing = "hello"
"#;
        let config: RescriptCodegenConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.root_module, "Stdb");
    }

    #[test]
    fn test_invalid_call_mode() {
        let toml_str = r#"call_mode = "invalid""#;
        let result: Result<RescriptCodegenConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown call_mode"));
    }

    #[test]
    fn test_load_config_defaults_when_no_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let config = load_config(&[temp.path()]).unwrap();
        assert_eq!(config.root_module, "Stdb");
        assert_eq!(config.call_mode, CallMode::Combined);
    }

    #[test]
    fn test_load_config_reads_file() {
        let temp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            temp.path().join(CONFIG_FILENAME),
            r#"root_module = "VR"
call_mode = "hooks"
"#,
        )
        .unwrap();
        let config = load_config(&[temp.path()]).unwrap();
        assert_eq!(config.root_module, "VR");
        assert_eq!(config.call_mode, CallMode::Hooks);
    }

    #[test]
    fn test_load_config_first_path_wins() {
        let dir1 = tempfile::TempDir::new().unwrap();
        let dir2 = tempfile::TempDir::new().unwrap();
        std::fs::write(dir1.path().join(CONFIG_FILENAME), r#"root_module = "First""#).unwrap();
        std::fs::write(dir2.path().join(CONFIG_FILENAME), r#"root_module = "Second""#).unwrap();
        let config = load_config(&[dir1.path(), dir2.path()]).unwrap();
        assert_eq!(config.root_module, "First");
    }

    #[test]
    fn test_load_config_falls_through_to_second_path() {
        let dir1 = tempfile::TempDir::new().unwrap();
        let dir2 = tempfile::TempDir::new().unwrap();
        // No file in dir1
        std::fs::write(dir2.path().join(CONFIG_FILENAME), r#"root_module = "Second""#).unwrap();
        let config = load_config(&[dir1.path(), dir2.path()]).unwrap();
        assert_eq!(config.root_module, "Second");
    }

    #[test]
    fn test_default_output_dir_strategy_is_subdirectories() {
        let config = RescriptCodegenConfig::default();
        assert_eq!(config.output_dir_strategy, OutputDirStrategy::Subdirectories);
    }

    #[test]
    fn test_parse_output_dir_strategy_flat() {
        let toml_str = r#"output_dir_strategy = "flat""#;
        let config: RescriptCodegenConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.output_dir_strategy, OutputDirStrategy::Flat);
    }

    #[test]
    fn test_parse_output_dir_strategy_subdirectories() {
        let toml_str = r#"output_dir_strategy = "subdirectories""#;
        let config: RescriptCodegenConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.output_dir_strategy, OutputDirStrategy::Subdirectories);
    }

    #[test]
    fn test_invalid_output_dir_strategy() {
        let toml_str = r#"output_dir_strategy = "nested""#;
        let result: Result<RescriptCodegenConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }
}
