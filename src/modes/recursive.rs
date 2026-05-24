use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::format::fmt_thousands;
use crate::output::table::{print_child_dir_rows, print_recursive_dir};
use crate::tokenize_files::{FileResult, Outcome};

/// Group results by their parent directory relative to root.
/// Returns a BTreeMap: dir_rel -> list of results for files directly in that dir.
fn group_by_dir<'a>(
    root: &Path,
    results: &'a [FileResult],
) -> BTreeMap<PathBuf, Vec<&'a FileResult>> {
    let mut map: BTreeMap<PathBuf, Vec<&FileResult>> = BTreeMap::new();
    for r in results {
        let parent = r.entry.rel.parent().unwrap_or(Path::new(".")).to_path_buf();
        let key = if parent == Path::new("") {
            PathBuf::from(".")
        } else {
            parent
        };
        map.entry(key).or_default().push(r);
    }
    // ensure every dir ancestor is present so we can compute child roll-ups
    let _ = root; // root used indirectly
    map
}

/// Compute rolled-up token total for a directory subtree.
fn subtree_tokens(dir: &Path, all_results: &[FileResult]) -> usize {
    all_results
        .iter()
        .map(|r| {
            if let Outcome::Tokens(n) = r.outcome {
                let parent = r.entry.rel.parent().unwrap_or(Path::new("."));
                let parent_key = if parent == Path::new("") {
                    PathBuf::from(".")
                } else {
                    parent.to_path_buf()
                };
                if parent_key.starts_with(dir) || parent_key == dir {
                    return n;
                }
            }
            0
        })
        .sum()
}

pub fn run_recursive(results: &[FileResult], with_dirs: bool) {
    let groups = group_by_dir(Path::new("."), results);

    let mut grand_total = 0usize;

    for (dir, dir_results) in &groups {
        // only print dirs that have at least one file with tokens
        let has_tokens = dir_results
            .iter()
            .any(|r| matches!(r.outcome, Outcome::Tokens(_)));
        if !has_tokens {
            continue;
        }

        let label = dir.to_string_lossy();
        print_recursive_dir(&label, dir_results);

        if with_dirs {
            // find immediate child subdirectories
            let child_dirs = child_subdirs(dir, results);
            if !child_dirs.is_empty() {
                let child_rows: Vec<(String, usize)> = child_dirs
                    .iter()
                    .map(|c| {
                        let tokens = subtree_tokens(c, results);
                        (c.to_string_lossy().into_owned(), tokens)
                    })
                    .collect();
                print_child_dir_rows(&child_rows);
            }
        }

        let dir_total: usize = dir_results
            .iter()
            .map(|r| match r.outcome {
                Outcome::Tokens(n) => n,
                _ => 0,
            })
            .sum();
        grand_total += dir_total;
    }

    println!("GRAND TOTAL: {}", fmt_thousands(grand_total));
}

/// Return immediate child subdirectory paths (relative) for a given dir.
fn child_subdirs(dir: &Path, results: &[FileResult]) -> Vec<PathBuf> {
    let mut seen = std::collections::BTreeSet::new();
    for r in results {
        let rel = &r.entry.rel;
        let parent = rel.parent().unwrap_or(Path::new("."));
        let parent_key = if parent == Path::new("") {
            PathBuf::from(".")
        } else {
            parent.to_path_buf()
        };

        // is parent_key a direct child of dir?
        if let Ok(suffix) = parent_key.strip_prefix(dir) {
            let mut comps = suffix.components();
            if let Some(first) = comps.next() {
                if comps.next().is_none() {
                    // exactly one level deeper
                    let child = dir.join(first);
                    if child != dir {
                        seen.insert(child);
                    }
                }
            }
        }
    }
    seen.into_iter().collect()
}
