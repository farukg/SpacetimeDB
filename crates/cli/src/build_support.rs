use std::io;
use std::path::{Path, PathBuf};

/// Enumerate source files while honoring repository and template ignore rules.
pub fn list_source_files(root_dir: &Path, repo_root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in ignore::WalkBuilder::new(root_dir)
        .standard_filters(true)
        .hidden(false)
        .require_git(false)
        .build()
    {
        let entry = entry.map_err(|error| io::Error::other(error.to_string()))?;
        if entry.file_type().is_some_and(|file_type| file_type.is_file()) {
            files.push(make_repo_root_relative(entry.path(), repo_root)?);
        }
    }
    files.sort();
    Ok(files)
}

fn make_repo_root_relative(full_path: &Path, repo_root: &Path) -> io::Result<PathBuf> {
    full_path.strip_prefix(repo_root).map(Path::to_path_buf).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Path {} is outside repo root {}", full_path.display(), repo_root.display()),
        )
    })
}
