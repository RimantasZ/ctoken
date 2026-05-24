use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

use crate::config::Profile;
use crate::error::{CtokenError, Result};

pub struct FileEntry {
    pub abs: PathBuf,
    pub rel: PathBuf,
}

pub struct Selection<'a> {
    pub gitignore: bool,
    pub match_globs: &'a [String],
    pub profile: Option<&'a Profile>,
}

fn build_globset(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        let glob = Glob::new(p)
            .map_err(|e| CtokenError::usage(format!("invalid glob pattern '{}': {}", p, e)))?;
        builder.add(glob);
    }
    let set = builder
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build glob set: {}", e))?;
    Ok(Some(set))
}

fn build_profile_globsets(profile: &Profile) -> Result<(Option<GlobSet>, Option<GlobSet>)> {
    let include = build_globset(&profile.include)?;
    let exclude = build_globset(&profile.exclude)?;
    Ok((include, exclude))
}

pub fn collect(root: &Path, sel: &Selection<'_>) -> Result<Vec<FileEntry>> {
    use crate::binary::is_binary_ext;

    let profile_sets = sel.profile.map(build_profile_globsets).transpose()?;
    let match_set = build_globset(sel.match_globs)?;

    let walker = WalkBuilder::new(root)
        .git_ignore(sel.gitignore)
        .git_global(sel.gitignore)
        .git_exclude(sel.gitignore)
        .hidden(false)
        .follow_links(false)
        .filter_entry(|entry| {
            // always exclude .git directory
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                return entry.file_name() != ".git";
            }
            true
        })
        .build();

    let mut entries = Vec::new();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(e) => {
                eprintln!("warning: walk error: {}", e);
                continue;
            }
        };

        let ft = match entry.file_type() {
            Some(ft) => ft,
            None => continue,
        };
        if !ft.is_file() {
            continue;
        }

        let abs = entry.path().to_path_buf();
        let rel = abs.strip_prefix(root).unwrap_or(&abs).to_path_buf();

        // binary extension filter
        if is_binary_ext(&abs) {
            continue;
        }

        // profile filter
        if let Some((ref inc_set, ref exc_set)) = profile_sets {
            if let Some(ref inc) = inc_set {
                if !inc.is_match(&rel) {
                    continue;
                }
            }
            if let Some(ref exc) = exc_set {
                if exc.is_match(&rel) {
                    continue;
                }
            }
        }

        // match glob filter — all globs must match
        if let Some(ref ms) = match_set {
            if !ms.is_match(&rel) {
                continue;
            }
        }

        entries.push(FileEntry { abs, rel });
    }

    entries.sort_by(|a, b| a.rel.cmp(&b.rel));

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn basic_collect() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn foo() {}").unwrap();

        let sel = Selection {
            gitignore: false,
            match_globs: &[],
            profile: None,
        };
        let entries = collect(dir.path(), &sel).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn gitignore_excludes_files() {
        let dir = tempdir().unwrap();
        // ignore crate requires a .git dir to apply gitignore rules
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(dir.path().join("included.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("ignored.txt"), "secret").unwrap();

        let sel = Selection {
            gitignore: true,
            match_globs: &[],
            profile: None,
        };
        let entries = collect(dir.path(), &sel).unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.rel.to_str().unwrap()).collect();
        assert!(names.contains(&"included.rs"));
        assert!(!names.contains(&"ignored.txt"));
    }

    #[test]
    fn gitignore_off_includes_all() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(dir.path().join("included.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("ignored.txt"), "secret").unwrap();

        let sel = Selection {
            gitignore: false,
            match_globs: &[],
            profile: None,
        };
        let entries = collect(dir.path(), &sel).unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.rel.to_str().unwrap()).collect();
        assert!(names.contains(&"ignored.txt"));
    }

    #[test]
    fn git_dir_always_excluded() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".git").join("config"), "[core]").unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

        let sel = Selection {
            gitignore: false,
            match_globs: &[],
            profile: None,
        };
        let entries = collect(dir.path(), &sel).unwrap();
        assert!(entries.iter().all(|e| !e.rel.starts_with(".git")));
    }

    #[test]
    fn match_glob_filter() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("README.md"), "# hi").unwrap();

        let globs = vec!["**/*.md".to_string()];
        let sel = Selection {
            gitignore: false,
            match_globs: &globs,
            profile: None,
        };
        let entries = collect(dir.path(), &sel).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].rel.to_str().unwrap(), "README.md");
    }

    #[test]
    fn binary_ext_excluded() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("image.png"), b"\x89PNG").unwrap();

        let sel = Selection {
            gitignore: false,
            match_globs: &[],
            profile: None,
        };
        let entries = collect(dir.path(), &sel).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].rel.to_str().unwrap(), "main.rs");
    }
}
