use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::format::fmt_thousands;
use crate::tokenize_files::{FileResult, Outcome};

/// Print a cloc-style table: header, dash separator, data rows, dash separator, total row.
/// `right_cols` is a bitmask of which column indices are right-aligned (0 = left).
fn print_table(headers: &[&str], right_cols: &[bool], rows: &[Vec<String>], total: Vec<String>) {
    // compute column widths as max of header, all data cells, and total cell
    let ncols = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows.iter().chain(std::iter::once(&total)) {
        for (i, cell) in row.iter().enumerate() {
            if i < ncols {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let sep_width: usize = widths.iter().sum::<usize>() + widths.len().saturating_sub(1) * 2;
    let sep = "-".repeat(sep_width);

    let fmt_row = |row: &[String]| -> String {
        row.iter()
            .enumerate()
            .map(|(i, cell)| {
                let w = widths[i];
                if right_cols.get(i).copied().unwrap_or(false) {
                    format!("{:>w$}", cell)
                } else {
                    format!("{:<w$}", cell)
                }
            })
            .collect::<Vec<_>>()
            .join("  ")
    };

    let header_str: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let w = widths[i];
            if right_cols.get(i).copied().unwrap_or(false) {
                format!("{:>w$}", h)
            } else {
                format!("{:<w$}", h)
            }
        })
        .collect::<Vec<_>>()
        .join("  ");

    println!("{header_str}");
    println!("{sep}");
    for row in rows {
        println!("{}", fmt_row(row));
    }
    println!("{sep}");
    println!("{}", fmt_row(&total));
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
                    ".".to_string()
                } else {
                    comp.as_os_str().to_string_lossy().into_owned()
                }
            }
            None => ".".to_string(),
        };
        *groups.entry(key).or_default() += n;
    }

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
    let rows: Vec<Vec<String>> = keys
        .iter()
        .map(|key| {
            let n = groups[key];
            total += n;
            vec![key.clone(), fmt_thousands(n)]
        })
        .collect();

    print_table(
        &["DIRECTORY", "TOKENS"],
        &[false, true],
        &rows,
        vec!["TOTAL".into(), fmt_thousands(total)],
    );
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

    let mut entries: Vec<_> = groups.into_iter().collect();
    entries.sort_by_key(|b| std::cmp::Reverse(b.1.tokens));

    let mut total_files = 0usize;
    let mut total_tokens = 0usize;
    let rows: Vec<Vec<String>> = entries
        .iter()
        .map(|(ext, stats)| {
            total_files += stats.files;
            total_tokens += stats.tokens;
            vec![
                ext.clone(),
                fmt_thousands(stats.files),
                fmt_thousands(stats.tokens),
            ]
        })
        .collect();

    print_table(
        &["TYPE", "FILES", "TOKENS"],
        &[false, true, true],
        &rows,
        vec![
            "TOTAL".into(),
            fmt_thousands(total_files),
            fmt_thousands(total_tokens),
        ],
    );
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

    let mut subtotal = 0usize;
    let rows: Vec<Vec<String>> = entries
        .iter()
        .map(|(ext, stats)| {
            subtotal += stats.tokens;
            vec![
                ext.clone(),
                fmt_thousands(stats.files),
                fmt_thousands(stats.tokens),
            ]
        })
        .collect();

    print_table(
        &["TYPE", "FILES", "TOKENS"],
        &[false, true, true],
        &rows,
        vec!["SUBTOTAL".into(), String::new(), fmt_thousands(subtotal)],
    );
}

/// Child directory rows for --recursive-with-dir mode.
pub fn print_child_dir_rows(child_dirs: &[(String, usize)]) {
    if child_dirs.is_empty() {
        return;
    }
    let rows: Vec<Vec<String>> = child_dirs
        .iter()
        .map(|(name, tokens)| vec![name.clone(), fmt_thousands(*tokens)])
        .collect();
    let total_tokens: usize = child_dirs.iter().map(|(_, t)| t).sum();

    print_table(
        &["SUBDIRECTORY", "TOKENS"],
        &[false, true],
        &rows,
        vec!["TOTAL".into(), fmt_thousands(total_tokens)],
    );
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
