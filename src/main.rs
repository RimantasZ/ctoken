mod binary;
mod cli;
mod config;
mod error;
mod format;
mod modes;
mod output;
mod path;
mod tokenize;
mod tokenize_files;
mod walk;

use clap::Parser;
use std::io::{self, BufRead, Write};

use cli::Cli;
use error::{CtokenError, Result};
use path::Target;
use tokenize::Tokenizer;
use tokenize_files::{tokenize_all, total_tokens, Outcome};
use walk::Selection;

fn main() {
    let code = match run() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {}", e);
            e.exit_code()
        }
    };
    std::process::exit(code);
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    cli.validate()?;

    // Handle --recreate-profiles before anything else (path still required by clap)
    if cli.recreate_profiles {
        config::recreate(|msg| {
            eprint!("{}", msg);
            io::stderr().flush().ok();
            let stdin = io::stdin();
            let line = stdin
                .lock()
                .lines()
                .next()
                .unwrap_or(Ok(String::new()))
                .unwrap_or_default();
            matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
        })?;
        return Ok(());
    }

    // Initialize profile config on every run
    let profile_file = config::load_or_init()?;

    // Resolve the profile if requested
    let profile = match &cli.profile {
        Some(name) => {
            let p = profile_file.profiles.get(name).ok_or_else(|| {
                let available: Vec<_> = profile_file.profiles.keys().cloned().collect();
                CtokenError::usage(format!(
                    "profile '{}' not found; available: {}",
                    name,
                    available.join(", ")
                ))
            })?;
            Some(p)
        }
        None => None,
    };

    let tokenizer = Tokenizer::new(&cli.encoding)?;

    match path::classify(&cli.path)? {
        Target::File(abs) => run_single_file(&cli, &abs, &tokenizer),
        Target::Dir(abs) => run_directory(&cli, &abs, &tokenizer, profile),
    }
}

fn run_single_file(cli: &Cli, abs: &std::path::Path, tokenizer: &Tokenizer) -> Result<()> {
    use binary::{is_binary_content, is_binary_ext};

    let bytes =
        std::fs::read(abs).map_err(|e| anyhow::anyhow!("cannot read {}: {}", abs.display(), e))?;

    let is_bin = is_binary_ext(abs) || is_binary_content(&bytes);

    let token_count = if is_bin {
        0
    } else {
        match String::from_utf8(bytes) {
            Ok(text) => tokenizer.count(&text),
            Err(_) => {
                eprintln!(
                    "warning: {} contains invalid UTF-8, skipping",
                    abs.display()
                );
                0
            }
        }
    };

    let rel = abs.file_name().map(std::path::Path::new).unwrap_or(abs);

    if cli.verbose {
        if is_bin {
            println!("{}", format::fmt_verbose_line(rel, &Outcome::Binary));
        } else {
            println!(
                "{}",
                format::fmt_verbose_line(rel, &Outcome::Tokens(token_count))
            );
        }
    }

    if cli.json {
        let v = output::json::single_file_json(abs, token_count);
        println!("{}", serde_json::to_string(&v).unwrap());
    } else {
        println!("{}", token_count);
    }

    Ok(())
}

fn run_directory(
    cli: &Cli,
    root: &std::path::Path,
    tokenizer: &Tokenizer,
    profile: Option<&config::Profile>,
) -> Result<()> {
    let sel = Selection {
        gitignore: cli.gitignore_on(),
        match_globs: &cli.match_globs,
        profile,
    };

    let files = walk::collect(root, &sel)?;
    let results = tokenize_all(files, tokenizer);

    // verbose output before table/json
    if cli.verbose {
        output::verbose::print_verbose(&results);
    }

    if cli.sum {
        let total = total_tokens(&results);
        if cli.json {
            let v = output::json::sum_json(total);
            println!("{}", serde_json::to_string(&v).unwrap());
        } else {
            println!("{}", total);
        }
        return Ok(());
    }

    if cli.recursive || cli.recursive_with_dir {
        // --json already rejected by cli.validate()
        modes::recursive::run_recursive(&results, cli.recursive_with_dir);
        return Ok(());
    }

    if cli.by_type {
        if cli.json {
            let v = output::json::by_type_json(root, &results);
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
        } else {
            output::table::print_by_type(&results);
        }
        return Ok(());
    }

    // default mode
    if cli.json {
        let v = output::json::default_json(root, &results);
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    } else {
        output::table::print_default(root, &results);
    }

    Ok(())
}
