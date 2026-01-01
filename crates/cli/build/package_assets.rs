use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

pub enum TemplateSources {
    Repository(PathBuf),
    Package(PathBuf),
}

impl TemplateSources {
    pub fn prepare(manifest: &Path, out_dir: &Path) -> io::Result<Self> {
        let archive = manifest.join("build/package-assets.tar");
        println!("cargo:rerun-if-changed={}", archive.display());
        if archive.try_exists()? {
            let root = out_dir.join("package-assets");
            if root.try_exists()? {
                fs::remove_dir_all(&root)?;
            }
            fs::create_dir_all(&root)?;
            tar::Archive::new(File::open(archive)?).unpack(&root)?;
            Ok(Self::Package(root.canonicalize()?))
        } else {
            Ok(Self::Repository(manifest.join("../..").canonicalize()?))
        }
    }

    pub fn root(&self) -> &Path {
        match self {
            Self::Repository(root) => root,
            Self::Package(root) => root,
        }
    }
}
