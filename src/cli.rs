use clap::{Parser, ValueEnum};
use std::path::PathBuf;

use crate::error::{CtokenError, Result};

#[derive(Parser, Debug)]
#[command(name = "ctoken", version, about = "Count tokens in project files")]
pub struct Cli {
    /// Path to a file or directory to tokenize
    pub path: PathBuf,

    /// Group by file extension instead of by subdirectory
    #[arg(short = 't', long = "type")]
    pub by_type: bool,

    /// Honor .gitignore (default: on)
    #[arg(short = 'g', long, default_value = "on", value_name = "on|off")]
    pub gitignore: GitignoreFlag,

    /// Glob pattern restricting included files (repeatable)
    #[arg(short = 'm', long = "match", value_name = "GLOB", action = clap::ArgAction::Append)]
    pub match_globs: Vec<String>,

    /// Use named profile from ~/.config/ctoken/profiles.toml; omit NAME to list available profiles
    #[arg(short = 'p', long, value_name = "NAME", num_args = 0..=1, default_missing_value = "")]
    pub profile: Option<String>,

    /// Rewrite built-in profile entries in profiles.toml (interactive)
    #[arg(long)]
    pub recreate_profiles: bool,

    /// Walk recursively; per-directory table grouped by file type; print totals at end
    #[arg(long)]
    pub recursive: bool,

    /// Same as --recursive, but each directory also includes child directory counts
    #[arg(long)]
    pub recursive_with_dir: bool,

    /// Log each file processed
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Print only the grand total (single integer)
    #[arg(short = 's', long = "sum")]
    pub sum: bool,

    /// Emit JSON instead of a table (incompatible with --recursive / --recursive-with-dir)
    #[arg(long)]
    pub json: bool,

    /// Tiktoken encoding to use
    #[arg(long, default_value = "o200k-base", value_name = "NAME")]
    pub encoding: EncodingArg,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum GitignoreFlag {
    On,
    Off,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum EncodingArg {
    #[value(name = "cl100k_base", alias = "cl100k-base")]
    Cl100kBase,
    #[value(name = "o200k_base", alias = "o200k-base")]
    O200kBase,
    #[value(name = "p50k_base", alias = "p50k-base")]
    P50kBase,
    #[value(name = "p50k_edit", alias = "p50k-edit")]
    P50kEdit,
    #[value(name = "r50k_base", alias = "r50k-base")]
    R50kBase,
}

impl std::fmt::Display for EncodingArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodingArg::Cl100kBase => write!(f, "cl100k_base"),
            EncodingArg::O200kBase => write!(f, "o200k_base"),
            EncodingArg::P50kBase => write!(f, "p50k_base"),
            EncodingArg::P50kEdit => write!(f, "p50k_edit"),
            EncodingArg::R50kBase => write!(f, "r50k_base"),
        }
    }
}

impl Cli {
    pub fn validate(&self) -> Result<()> {
        if self.json && (self.recursive || self.recursive_with_dir) {
            return Err(CtokenError::usage(
                "--json cannot be combined with --recursive / --recursive-with-dir",
            ));
        }
        if self.recursive && self.recursive_with_dir {
            return Err(CtokenError::usage(
                "--recursive and --recursive-with-dir are mutually exclusive; use one",
            ));
        }
        Ok(())
    }

    pub fn gitignore_on(&self) -> bool {
        self.gitignore == GitignoreFlag::On
    }
}
