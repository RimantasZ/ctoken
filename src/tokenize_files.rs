use std::fs;

use rayon::prelude::*;

use crate::binary::{is_binary_content, is_binary_ext};
use crate::tokenize::Tokenizer;
use crate::walk::FileEntry;

#[derive(Debug)]
pub enum Outcome {
    Tokens(usize),
    Binary,
    #[allow(dead_code)]
    Error(String),
}

pub struct FileResult {
    pub entry: FileEntry,
    pub outcome: Outcome,
}

pub fn tokenize_all(files: Vec<FileEntry>, tokenizer: &Tokenizer) -> Vec<FileResult> {
    files
        .into_par_iter()
        .map(|entry| {
            let outcome = process_file(&entry, tokenizer);
            FileResult { entry, outcome }
        })
        .collect::<Vec<_>>()
        // sort by relative path for stable output
        .into_iter()
        .collect::<Vec<_>>()
        .tap_sort()
}

trait TapSort {
    fn tap_sort(self) -> Self;
}

impl TapSort for Vec<FileResult> {
    fn tap_sort(mut self) -> Self {
        self.sort_by(|a, b| a.entry.rel.cmp(&b.entry.rel));
        self
    }
}

fn process_file(entry: &FileEntry, tokenizer: &Tokenizer) -> Outcome {
    let bytes = match fs::read(&entry.abs) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("warning: cannot read {}: {}", entry.abs.display(), e);
            return Outcome::Error(e.to_string());
        }
    };

    // content-based binary detection (extension was already checked in walker)
    if !is_binary_ext(&entry.abs) && is_binary_content(&bytes) {
        return Outcome::Binary;
    }

    // shouldn't reach here for ext-binary files (walker excludes them),
    // but guard anyway
    if is_binary_ext(&entry.abs) {
        return Outcome::Binary;
    }

    let text = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            eprintln!(
                "warning: cannot read {}: invalid UTF-8",
                entry.abs.display()
            );
            return Outcome::Error("invalid UTF-8".into());
        }
    };

    Outcome::Tokens(tokenizer.count(&text))
}

pub fn total_tokens(results: &[FileResult]) -> usize {
    results
        .iter()
        .map(|r| match r.outcome {
            Outcome::Tokens(n) => n,
            _ => 0,
        })
        .sum()
}
