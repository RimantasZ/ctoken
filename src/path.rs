use std::path::{Path, PathBuf};

use crate::error::{CtokenError, Result};

pub enum Target {
    File(PathBuf),
    Dir(PathBuf),
}

pub fn classify(path: &Path) -> Result<Target> {
    let canonical = path
        .canonicalize()
        .map_err(|_| CtokenError::usage(format!("{}: not a file or directory", path.display())))?;

    let meta = canonical
        .metadata()
        .map_err(|_| CtokenError::usage(format!("{}: not a file or directory", path.display())))?;

    if meta.is_file() {
        Ok(Target::File(canonical))
    } else if meta.is_dir() {
        Ok(Target::Dir(canonical))
    } else {
        Err(CtokenError::usage(format!(
            "{}: not a file or directory",
            path.display()
        )))
    }
}
