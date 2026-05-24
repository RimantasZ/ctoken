use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use comfy_table::{Cell, CellAlignment, Table};

use crate::format::fmt_thousands;
use crate::tokenize_files::{FileResult, Outcome};

fn right(s: impl ToString) -> Cell {
    Cell::new(s).set_alignment(CellAlignment::Right)
}

/// Default mode: group by immediate subdirectory under root.
pub fn print_default(_root: &Path, results: &[FileResult]) {
    let mut groups: BTreeMap<String, usize> = BTreeMap::new();

    for r in results {
        let n = match r.outcome {
            Outcome::Tokens(n) => n,
            _ => continue,
        };
        let key = match r.entry.rel.components().next() {
            Some(comp) => {
                let comp_path = PathBuf::from(comp.as_os_str());
                if comp_path == r.entry.rel {
                    // file directly in root
                    ".".to_string()
                } else {
                    comp.as_os_str().to_string_lossy().into_owned()
                }
            }
            None => ".".to_string(),
        };
        *groups.entry(key).or_default() += n;
    }

    let mut table = Table::new();
    table.set_header(vec!["DIRECTORY", "TOKENS"]);

    // "." first, then alphabetical
    let mut keys: Vec<_> = groups.keys().cloned().collect();
    keys.sort_by(|a, b| {
        if a == "." {
            std::cmp::Ordering::Less
        } else if b == "." {
            std::cmp::Ordering::Greater
        } else {
            a.cmp(b)
        }
    });

    let mut total = 0usize;
    for key in &keys {
        let n = groups[key];
        total += n;
        table.add_row(vec![Cell::new(key), right(fmt_thousands(n))]);
    }
    table.add_row(vec![Cell::new("TOTAL"), right(fmt_thousands(total))]);

    println!("{table}");
}

/// -t mode: group by file extension across whole tree.
pub fn print_by_type(results: &[FileResult]) {
    #[derive(Default)]
    struct TypeStats {
        files: usize,
        tokens: usize,
    }

    let mut groups: BTreeMap<String, TypeStats> = BTreeMap::new();

    for r in results {
        let n = match r.outcome {
            Outcome::Tokens(n) => n,
            _ => continue,
        };
        let ext = r
            .entry
            .rel
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_else(|| "(none)".into());
        let stats = groups.entry(ext).or_default();
        stats.files += 1;
        stats.tokens += n;
    }

    // sort by tokens desc
    let mut entries: Vec<_> = groups.into_iter().collect();
    entries.sort_by_key(|b| std::cmp::Reverse(b.1.tokens));

    let mut table = Table::new();
    table.set_header(vec!["TYPE", "FILES", "TOKENS"]);

    let mut total_files = 0usize;
    let mut total_tokens = 0usize;
    for (ext, stats) in &entries {
        total_files += stats.files;
        total_tokens += stats.tokens;
        table.add_row(vec![
            Cell::new(ext),
            right(fmt_thousands(stats.files)),
            right(fmt_thousands(stats.tokens)),
        ]);
    }
    table.add_row(vec![
        Cell::new("TOTAL"),
        right(fmt_thousands(total_files)),
        right(fmt_thousands(total_tokens)),
    ]);

    println!("{table}");
}

/// Per-directory sub-table for recursive mode (files only, by extension).
pub fn print_recursive_dir(dir_label: &str, dir_results: &[&FileResult]) {
    #[derive(Default)]
    struct TypeStats {
        files: usize,
        tokens: usize,
    }

    let mut groups: BTreeMap<String, TypeStats> = BTreeMap::new();
    for r in dir_results {
        let n = match r.outcome {
            Outcome::Tokens(n) => n,
            _ => continue,
        };
        let ext = r
            .entry
            .rel
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_else(|| "(none)".into());
        let stats = groups.entry(ext).or_default();
        stats.files += 1;
        stats.tokens += n;
    }

    let mut entries: Vec<_> = groups.into_iter().collect();
    entries.sort_by_key(|b| std::cmp::Reverse(b.1.tokens));

    println!("=== {} ===", dir_label);

    let mut table = Table::new();
    table.set_header(vec!["TYPE", "FILES", "TOKENS"]);

    let mut subtotal = 0usize;
    for (ext, stats) in &entries {
        subtotal += stats.tokens;
        table.add_row(vec![
            Cell::new(ext),
            right(fmt_thousands(stats.files)),
            right(fmt_thousands(stats.tokens)),
        ]);
    }
    table.add_row(vec![
        Cell::new("SUBTOTAL"),
        Cell::new(""),
        right(fmt_thousands(subtotal)),
    ]);

    println!("{table}");
}

/// Child directory rows for --recursive-with-dir mode.
pub fn print_child_dir_rows(child_dirs: &[(String, usize)]) {
    if child_dirs.is_empty() {
        return;
    }
    let mut table = Table::new();
    table.set_header(vec!["SUBDIRECTORY", "TOKENS"]);
    for (name, tokens) in child_dirs {
        table.add_row(vec![Cell::new(name), right(fmt_thousands(*tokens))]);
    }
    println!("{table}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::tokenize_files::FileResult;
    use crate::walk::FileEntry;

    fn make_result(rel: &str, tokens: usize) -> FileResult {
        FileResult {
            entry: FileEntry {
                abs: PathBuf::from("/root").join(rel),
                rel: PathBuf::from(rel),
            },
            outcome: Outcome::Tokens(tokens),
        }
    }

    #[test]
    fn default_groups_by_subdir() {
        let results = vec![
            make_result("src/main.rs", 100),
            make_result("src/lib.rs", 200),
            make_result("README.md", 50),
        ];
        // just verify it doesn't panic
        print_default(Path::new("/root"), &results);
    }

    #[test]
    fn by_type_groups_extensions() {
        let results = vec![
            make_result("a.rs", 100),
            make_result("b.rs", 200),
            make_result("c.md", 50),
        ];
        print_by_type(&results);
    }
}
