use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

#[path = "../src/build_support.rs"]
mod build_support;
#[path = "package_assets.rs"]
mod package_assets;
use package_assets::TemplateSources;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let sources = TemplateSources::prepare(&manifest_dir, &out_dir)?;
    let git_hash = find_git_hash();
    println!("cargo:rustc-env=GIT_HASH={git_hash}");

    generate_template_files(&sources);
    Ok(())
}

fn nix_injected_commit_hash() -> Option<String> {
    use std::env::VarError;
    // Our flake.nix sets this environment variable to be our git commit hash during the build.
    // This is important because git metadata is otherwise not available within the nix build sandbox,
    // and we don't install the git command-line tool in our build.
    match std::env::var("SPACETIMEDB_NIX_BUILD_GIT_COMMIT") {
        Ok(commit_sha) => {
            // Var is set, we're building under Nix.
            Some(commit_sha)
        }

        Err(VarError::NotPresent) => {
            // Var is not set, we're not in Nix.
            None
        }
        Err(VarError::NotUnicode(gross)) => {
            // Var is set but is invalid unicode, something is very wrong.
            panic!("Injected commit hash is not valid unicode: {gross:?}")
        }
    }
}

fn is_nix_build() -> bool {
    nix_injected_commit_hash().is_some()
}

fn find_git_hash() -> String {
    nix_injected_commit_hash().unwrap_or_else(|| {
        // When we're *not* building in Nix, we can assume that git metadata is still present in the filesystem,
        // and that the git command-line tool is installed.
        let output = Command::new("git").args(["rev-parse", "HEAD"]).output().unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    })
}

// This method generates functions with data used in `spacetime init`:
//
//   * `get_templates_json` - returns contents of the JSON file with the list of templates
//   * `get_template_files` - returns a HashMap with templates contents based on the
//                            templates list at templates/templates-list.json
//   * `get_skill` - returns the content of a skill file by name
fn generate_template_files(sources: &TemplateSources) {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("embedded_templates.rs");

    let repo_root = sources.root();
    let templates_dir = repo_root.join("templates");

    for entry in std::fs::read_dir(&templates_dir).expect("Failed to read templates") {
        let entry = entry.expect("Failed to read template entry");
        let metadata = entry.path().join(".template.json");
        if metadata.is_file() {
            println!("cargo:rerun-if-changed={}", metadata.display());
        }
    }

    let discovered_templates = discover_templates(&templates_dir);

    let mut generated_code = String::new();
    generated_code.push_str("use spacetimedb_data_structures::map::HashMap;\n\n");

    generated_code.push_str("pub fn get_templates_json() -> &'static str {\n");
    generated_code.push_str("    r#\"");
    generated_code.push_str(&generate_templates_json(&discovered_templates));
    generated_code.push_str("\"#\n");
    generated_code.push_str("}\n\n");

    generated_code
        .push_str("pub fn get_template_files() -> HashMap<&'static str, HashMap<&'static str, &'static str>> {\n");
    generated_code.push_str("    let mut templates = HashMap::new();\n\n");

    for template in &discovered_templates {
        if let Some(ref server_source) = template.server_source {
            generate_template_entry(&mut generated_code, server_source, sources);
        }

        if let Some(ref client_source) = template.client_source {
            generate_template_entry(&mut generated_code, client_source, sources);
        }
    }

    generated_code.push_str("    templates\n");
    generated_code.push_str("}\n\n");

    let workspace_cargo = repo_root.join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", workspace_cargo.display());

    let (workspace_edition, workspace_versions) =
        extract_workspace_metadata(&workspace_cargo).expect("Failed to extract workspace metadata");

    let ts_bindings_package = repo_root.join("crates/bindings-typescript/package.json");
    println!("cargo:rerun-if-changed={}", ts_bindings_package.display());
    let ts_bindings_version =
        extract_ts_bindings_version(&ts_bindings_package).expect("Failed to read TypeScript bindings version");

    // Embed skill files from skills/*/SKILL.md
    let skills_dir = repo_root.join("skills");
    let skill_names = discover_skill_names(&skills_dir);

    generated_code.push_str("pub fn get_skill(name: &str) -> Option<&'static str> {\n");
    generated_code.push_str("    match name {\n");
    for skill_name in &skill_names {
        let skill_path = skills_dir.join(skill_name).join("SKILL.md");
        generated_code.push_str(&format!(
            "        \"{}\" => Some(include_str!(\"{}\")),\n",
            skill_name,
            skill_path.to_str().unwrap().replace('\\', "\\\\")
        ));
        println!("cargo:rerun-if-changed={}", skill_path.display());
    }
    generated_code.push_str("        _ => None,\n");
    generated_code.push_str("    }\n");
    generated_code.push_str("}\n\n");

    // Expose workspace metadata so `spacetime init` can rewrite template manifests without hardcoding versions.
    generated_code.push_str("pub fn get_workspace_edition() -> &'static str {\n");
    generated_code.push_str(&format!("    \"{}\"\n", workspace_edition.escape_default()));
    generated_code.push_str("}\n\n");

    generated_code.push_str("pub fn get_workspace_dependency_version(name: &str) -> Option<&'static str> {\n");
    generated_code.push_str("    match name {\n");
    for (name, version) in &workspace_versions {
        generated_code.push_str(&format!(
            "        \"{}\" => Some(\"{}\"),\n",
            name.escape_default(),
            version.escape_default()
        ));
    }
    generated_code.push_str("        _ => None,\n");
    generated_code.push_str("    }\n");
    generated_code.push_str("}\n");

    generated_code.push('\n');
    generated_code.push_str("pub fn get_typescript_bindings_version() -> &'static str {\n");
    generated_code.push_str(&format!("    \"{}\"\n", ts_bindings_version.escape_default()));
    generated_code.push_str("}\n");

    write_if_changed(&dest_path, generated_code.as_bytes()).expect("Failed to write embedded_templates.rs");
}

#[derive(Debug, serde::Serialize)]
struct TemplateInfo {
    id: String,
    description: String,
    #[serde(serialize_with = "serialize_option_string_as_empty")]
    server_source: Option<String>,
    #[serde(serialize_with = "serialize_option_string_as_empty")]
    client_source: Option<String>,
    server_lang: Option<String>,
    client_lang: Option<String>,
    client_framework: Option<String>,
}

#[derive(serde::Serialize)]
struct TemplatesJson<'a> {
    templates: &'a [TemplateInfo],
}

#[derive(serde::Deserialize)]
struct TemplateMetadata {
    description: String,
    client_lang: Option<String>,
    client_framework: Option<String>,
    server_lang: Option<String>,
}

fn discover_templates(templates_dir: &Path) -> Vec<TemplateInfo> {
    let mut templates = Vec::new();

    let entries = match fs::read_dir(templates_dir) {
        Ok(entries) => entries,
        Err(_) => return templates,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let template_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        let metadata_path = path.join(".template.json");
        if !metadata_path.exists() {
            continue;
        }
        println!("cargo:rerun-if-changed={}", metadata_path.display());

        let metadata_content = match fs::read_to_string(&metadata_path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        let metadata: TemplateMetadata = match serde_json::from_str(&metadata_content) {
            Ok(meta) => meta,
            Err(_) => continue,
        };

        let server_source = if metadata.server_lang.is_some() {
            Some(format!("{}/spacetimedb", template_name))
        } else {
            None
        };

        let client_source = if metadata.client_lang.is_some() {
            Some(template_name.to_string())
        } else {
            None
        };

        if server_source.is_some() || client_source.is_some() {
            templates.push(TemplateInfo {
                id: template_name.to_string(),
                description: metadata.description,
                server_source,
                client_source,
                server_lang: metadata.server_lang,
                client_lang: metadata.client_lang,
                client_framework: metadata.client_framework,
            });
        }
    }

    templates.sort_by(|a, b| a.id.cmp(&b.id));
    templates
}

fn generate_templates_json(templates: &[TemplateInfo]) -> String {
    let payload = TemplatesJson { templates };
    serde_json::to_string_pretty(&payload).expect("Failed to serialize templates JSON")
}

fn serialize_option_string_as_empty<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(value.as_deref().unwrap_or(""))
}

fn generate_template_entry(code: &mut String, source: &str, sources: &TemplateSources) {
    let (git_files, resolved_base) = get_git_tracked_files(Path::new(source), sources);

    if git_files.is_empty() {
        panic!("Template '{}' has no git-tracked files! Check that the directory exists and contains files tracked by git.", source);
    }

    let repo_root = sources.root();

    code.push_str("    {\n");
    code.push_str("        let mut files = HashMap::new();\n");

    for file_path in git_files {
        // Example file_path: modules/chat-console-rs/src/lib.rs (relative to repo root)
        // Example resolved_base: modules/chat-console-rs
        // Example relative_path: src/lib.rs
        let relative_path = match file_path.strip_prefix(&resolved_base) {
            Ok(p) => p,
            Err(_) => {
                eprintln!(
                    "Warning: Could not strip prefix '{}' from '{}' for source '{}'",
                    resolved_base.display(),
                    file_path.display(),
                    source
                );
                continue;
            }
        };
        // Example: "src/lib.rs"
        let relative_str = relative_path.to_str().unwrap().replace("\\", "/");

        // Example: /Users/user/SpacetimeDB/modules/quickstart-chat/src/lib.rs
        let full_path = repo_root.join(&file_path);
        if full_path.exists() && full_path.is_file() {
            build_support::write_template_file(code, &mut io::stdout().lock(), &relative_str, &full_path, repo_root)
                .expect("Failed to declare and embed template source");
        }
    }

    code.push_str(&format!("        templates.insert(\"{}\", files);\n", source));
    code.push_str("    }\n\n");
}

/// Get a list of files tracked by git from a given directory
fn get_git_tracked_files(path: &Path, sources: &TemplateSources) -> (Vec<PathBuf>, PathBuf) {
    match sources {
        TemplateSources::Package(root) => list_all_files(path, root),
        TemplateSources::Repository(root) => {
            if is_nix_build() {
                list_all_files(path, root)
            } else {
                match get_git_tracked_files_via_cli(path, root) {
                    Some(result) => result,
                    None => list_all_files(path, root),
                }
            }
        }
    }
}

fn list_all_files(path: &Path, repo_root: &Path) -> (Vec<PathBuf>, PathBuf) {
    let template_root_absolute = get_full_path_within_manifest_dir(path, repo_root);

    let files = build_support::list_source_files(&template_root_absolute, repo_root).unwrap_or_else(|error| {
        panic!(
            "Failed to enumerate template files under {}: {error}",
            template_root_absolute.display()
        )
    });

    (files, make_repo_root_relative(&template_root_absolute, repo_root))
}

/// Treat `relative_path` as a relative path within the repo root's templates directory
/// and transform it into an absolute, canonical path.
fn get_full_path_within_manifest_dir(relative_path: &Path, repo_root: &Path) -> PathBuf {
    let full_path = repo_root.join("templates").join(relative_path);

    full_path.canonicalize().unwrap_or_else(|e| {
        panic!("Failed to canonicalize path {}: {}", full_path.display(), e);
    })
}

/// Transform `full_path` into a relative path within `repo_root`.
///
/// `full_path` and `repo_root` should both be canonical paths, as by [`Path::canonicalize`].
fn make_repo_root_relative(full_path: &Path, repo_root: &Path) -> PathBuf {
    full_path
        .strip_prefix(repo_root)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|_| {
            panic!(
                "Path {} is outside repo root {}",
                full_path.display(),
                repo_root.display()
            )
        })
}

fn get_git_tracked_files_via_cli(path: &Path, repo_root: &Path) -> Option<(Vec<PathBuf>, PathBuf)> {
    let resolved_path = make_repo_root_relative(&get_full_path_within_manifest_dir(path, repo_root), repo_root);

    let output = match Command::new("git")
        .args(["ls-files", resolved_path.to_str().unwrap()])
        .current_dir(repo_root)
        .output()
    {
        Ok(output) => output,
        Err(_) => return None,
    };

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).unwrap();
    let files: Vec<PathBuf> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect();

    Some((files, resolved_path))
}

fn extract_workspace_metadata(path: &Path) -> io::Result<(String, BTreeMap<String, String>)> {
    let content = fs::read_to_string(path)?;
    let parsed: Value = content
        .parse()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    let table = parsed
        .as_table()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "workspace manifest is not a table"))?;

    let workspace = table
        .get("workspace")
        .and_then(Value::as_table)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "workspace section missing"))?;

    let edition = workspace
        .get("package")
        .and_then(Value::as_table)
        .and_then(|pkg| pkg.get("edition"))
        .and_then(Value::as_str)
        .unwrap_or("2021")
        .to_string();

    let mut versions = BTreeMap::new();
    if let Some(deps) = workspace.get("dependencies").and_then(Value::as_table) {
        for (name, value) in deps {
            let version_opt = match value {
                Value::String(s) => Some(normalize_version(s)),
                Value::Table(table) => table.get("version").and_then(Value::as_str).map(normalize_version),
                _ => None,
            };

            if let Some(version) = version_opt {
                versions.insert(name.clone(), version);
            }
        }
    }

    Ok((edition, versions))
}

fn extract_ts_bindings_version(path: &Path) -> io::Result<String> {
    let content = fs::read_to_string(path)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    parsed
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing \"version\" field in TypeScript bindings package.json",
            )
        })
}

fn normalize_version(version: &str) -> String {
    version.trim().trim_start_matches('=').to_string()
}

fn write_if_changed(path: &Path, contents: &[u8]) -> io::Result<()> {
    match fs::read(path) {
        Ok(existing) if existing == contents => Ok(()),
        _ => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut file = fs::File::create(path)?;
            file.write_all(contents)
        }
    }
}

/// Discover skill directories under skills/. Each directory containing a SKILL.md
/// file is considered a skill. Returns sorted skill names.
fn discover_skill_names(skills_dir: &Path) -> Vec<String> {
    let mut names = Vec::new();

    let entries = match fs::read_dir(skills_dir) {
        Ok(entries) => entries,
        Err(_) => {
            panic!(
                "Could not read skills directory at {}. Ensure skills/ exists at the repo root.",
                skills_dir.display()
            );
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir()
            && path.join("SKILL.md").exists()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
        {
            names.push(name.to_string());
        }
    }

    names.sort();
    names
}
