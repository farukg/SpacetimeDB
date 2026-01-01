use std::io::{self, Write};
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

/// Watch and embed the same repository-owned file, including shared template licenses.
pub fn write_template_file(
    code: &mut String,
    rerun: &mut impl Write,
    relative: &str,
    file: &Path,
    repo_root: &Path,
) -> io::Result<()> {
    let source = file.canonicalize()?;
    let repo_root = repo_root.canonicalize()?;
    if !source.starts_with(&repo_root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "template source escapes its repository",
        ));
    }
    let source = source
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "template source path is not UTF-8"))?;
    writeln!(rerun, "cargo:rerun-if-changed={source}")?;
    code.push_str(&format!(
        "        files.insert({relative:?}, include_str!({source:?}));\n"
    ));
    Ok(())
}

fn make_repo_root_relative(full_path: &Path, repo_root: &Path) -> io::Result<PathBuf> {
    full_path.strip_prefix(repo_root).map(Path::to_path_buf).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Path {} is outside repo root {}",
                full_path.display(),
                repo_root.display()
            ),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn shared_template_license_is_watched_at_the_embedded_source() -> io::Result<()> {
        let directory = tempfile::tempdir()?;
        let template = directory.path().join("templates/basic-cpp");
        let licenses = directory.path().join("licenses");
        std::fs::create_dir_all(&template)?;
        std::fs::create_dir_all(&licenses)?;
        let license = licenses.join("apache2.txt");
        std::fs::write(&license, "shared legal text")?;
        let link = template.join("LICENSE");
        std::os::unix::fs::symlink("../../licenses/apache2.txt", &link)?;
        let mut code = String::new();
        let mut rerun = Vec::new();
        write_template_file(&mut code, &mut rerun, "LICENSE", &link, directory.path())?;
        let license = license.canonicalize()?;
        assert_eq!(
            String::from_utf8(rerun).unwrap(),
            format!("cargo:rerun-if-changed={}\n", license.display())
        );
        assert!(code.contains("files.insert(\"LICENSE\", include_str!("));
        assert!(code.contains(license.to_str().unwrap()));
        assert!(!code.contains(link.to_str().unwrap()));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn template_symlink_cannot_embed_a_source_outside_its_repository() -> io::Result<()> {
        let directory = tempfile::tempdir()?;
        let repository = directory.path().join("repository");
        std::fs::create_dir_all(&repository)?;
        let outside = directory.path().join("outside.txt");
        std::fs::write(&outside, "external material")?;
        let link = repository.join("LICENSE");
        std::os::unix::fs::symlink(&outside, &link)?;
        let mut code = String::new();
        let mut rerun = Vec::new();
        let error = write_template_file(&mut code, &mut rerun, "LICENSE", &link, &repository).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(code.is_empty());
        assert!(rerun.is_empty());
        Ok(())
    }
}
