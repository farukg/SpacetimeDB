use std::fs;

use spacetimedb_cli::build_support::list_source_files;
use tempfile::TempDir;

#[test]
fn source_archive_enumeration_excludes_ignored_files() {
    let temp_dir = TempDir::new().expect("temporary source archive");
    let root = temp_dir.path();
    fs::create_dir_all(root.join("templates/example/node_modules/pkg")).expect("ignored directory");
    fs::create_dir_all(root.join("templates/example/src")).expect("source directory");
    fs::write(root.join(".gitignore"), ".env\nnode_modules/\n").expect("ignore rules");
    fs::write(root.join("templates/example/.env"), "secret-like value").expect("ignored env");
    fs::write(root.join("templates/example/node_modules/pkg/index.js"), "artifact").expect("ignored artifact");
    fs::write(root.join("templates/example/src/main.ts"), "source").expect("source file");

    fs::write(root.join("templates/example/.env.example"), "PUBLIC_VALUE=example").expect("example env file");

    let files = list_source_files(&root.join("templates/example"), root).expect("enumerate source archive");

    assert_eq!(
        files,
        vec![
            std::path::PathBuf::from("templates/example/.env.example"),
            std::path::PathBuf::from("templates/example/src/main.ts"),
        ]
    );
}
