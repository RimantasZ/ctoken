use std::path::Path;

use serde_json::{json, Value};

use crate::tokenize_files::{FileResult, Outcome};

/// Default mode JSON shape.
pub fn default_json(root: &Path, results: &[FileResult]) -> Value {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

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

    let total: usize = groups.values().sum();
    let entries: Vec<Value> = keys
        .iter()
        .map(|k| json!({"path": k, "tokens": groups[k]}))
        .collect();

    json!({
        "root": root.to_string_lossy(),
        "total": total,
        "entries": entries,
    })
}

/// -t mode JSON shape.
pub fn by_type_json(root: &Path, results: &[FileResult]) -> Value {
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Stats {
        files: usize,
        tokens: usize,
    }
    let mut groups: BTreeMap<String, Stats> = BTreeMap::new();

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
        let s = groups.entry(ext).or_default();
        s.files += 1;
        s.tokens += n;
    }

    let mut entries: Vec<_> = groups.into_iter().collect();
    entries.sort_by_key(|b| std::cmp::Reverse(b.1.tokens));

    let total: usize = entries.iter().map(|(_, s)| s.tokens).sum();
    let entries_json: Vec<Value> = entries
        .iter()
        .map(|(ext, s)| json!({"type": ext, "files": s.files, "tokens": s.tokens}))
        .collect();

    json!({
        "root": root.to_string_lossy(),
        "total": total,
        "entries": entries_json,
    })
}

/// -s mode JSON shape.
pub fn sum_json(total: usize) -> Value {
    json!({"total": total})
}

/// Single-file JSON shape.
pub fn single_file_json(path: &Path, tokens: usize) -> Value {
    json!({"path": path.to_string_lossy(), "tokens": tokens})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::walk::FileEntry;
    use std::path::PathBuf;

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
    fn default_json_shape() {
        let results = vec![
            make_result("src/main.rs", 100),
            make_result("README.md", 50),
        ];
        let v = default_json(Path::new("/root"), &results);
        assert_eq!(v["total"], 150);
        assert!(v["entries"].is_array());
        let entries = v["entries"].as_array().unwrap();
        assert!(entries.iter().any(|e| e["path"] == "."));
        assert!(entries.iter().any(|e| e["path"] == "src"));
    }

    #[test]
    fn by_type_json_shape() {
        let results = vec![
            make_result("a.rs", 100),
            make_result("b.rs", 200),
            make_result("c.md", 50),
        ];
        let v = by_type_json(Path::new("/root"), &results);
        assert_eq!(v["total"], 350);
        let entries = v["entries"].as_array().unwrap();
        assert!(entries.iter().any(|e| e["type"] == "rs"));
    }

    #[test]
    fn sum_json_shape() {
        let v = sum_json(12345);
        assert_eq!(v["total"], 12345);
    }
}
