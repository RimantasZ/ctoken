use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProfileFile {
    #[serde(flatten)]
    pub profiles: BTreeMap<String, Profile>,
}

pub fn builtin_profiles() -> Vec<(String, Profile)> {
    vec![
        (
            "java".into(),
            Profile {
                include: vec![
                    "**/*.java".into(),
                    "**/*.kt".into(),
                    "pom.xml".into(),
                    "build.gradle".into(),
                    "build.gradle.kts".into(),
                    "settings.gradle".into(),
                    "settings.gradle.kts".into(),
                ],
                exclude: vec!["target/**".into(), "build/**".into(), ".gradle/**".into()],
            },
        ),
        (
            "c_cpp".into(),
            Profile {
                include: vec![
                    "**/*.c".into(),
                    "**/*.h".into(),
                    "**/*.cc".into(),
                    "**/*.cpp".into(),
                    "**/*.cxx".into(),
                    "**/*.hpp".into(),
                    "**/*.hh".into(),
                    "CMakeLists.txt".into(),
                    "**/*.cmake".into(),
                    "Makefile".into(),
                ],
                exclude: vec!["build/**".into(), "out/**".into()],
            },
        ),
        (
            "typescript".into(),
            Profile {
                include: vec![
                    "**/*.ts".into(),
                    "**/*.tsx".into(),
                    "**/*.js".into(),
                    "**/*.jsx".into(),
                    "**/*.mjs".into(),
                    "**/*.cjs".into(),
                    "package.json".into(),
                    "tsconfig.json".into(),
                ],
                exclude: vec![
                    "node_modules/**".into(),
                    "dist/**".into(),
                    "build/**".into(),
                    ".next/**".into(),
                ],
            },
        ),
        (
            "python".into(),
            Profile {
                include: vec![
                    "**/*.py".into(),
                    "**/*.pyi".into(),
                    "pyproject.toml".into(),
                    "setup.py".into(),
                    "setup.cfg".into(),
                    "requirements*.txt".into(),
                ],
                exclude: vec![
                    "__pycache__/**".into(),
                    ".venv/**".into(),
                    "venv/**".into(),
                    "dist/**".into(),
                    "build/**".into(),
                    "*.egg-info/**".into(),
                ],
            },
        ),
        (
            "rust".into(),
            Profile {
                include: vec!["**/*.rs".into(), "Cargo.toml".into(), "Cargo.lock".into()],
                exclude: vec!["target/**".into()],
            },
        ),
        (
            "go".into(),
            Profile {
                include: vec!["**/*.go".into(), "go.mod".into(), "go.sum".into()],
                exclude: vec!["vendor/**".into()],
            },
        ),
    ]
}

pub fn config_path() -> Result<PathBuf> {
    // Allow override via env var for testing
    if let Ok(dir) = std::env::var("CTOKEN_CONFIG_DIR") {
        return Ok(PathBuf::from(dir).join("profiles.toml"));
    }

    let base = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?
        .join(".config");

    Ok(base.join("ctoken").join("profiles.toml"))
}

pub fn load_or_init() -> Result<ProfileFile> {
    let path = config_path()?;

    if !path.exists() {
        match write_initial_profiles(&path) {
            Ok(()) => {
                eprintln!("notice: created profile config at {}", path.display());
            }
            Err(e) => {
                eprintln!(
                    "warning: could not create profile config at {}: {}",
                    path.display(),
                    e
                );
                // return built-ins in memory only
                let mut pf = ProfileFile::default();
                for (name, profile) in builtin_profiles() {
                    pf.profiles.insert(name, profile);
                }
                return Ok(pf);
            }
        }
        let mut pf = ProfileFile::default();
        for (name, profile) in builtin_profiles() {
            pf.profiles.insert(name, profile);
        }
        return Ok(pf);
    }

    let mut pf = load_file(&path)?;
    let mut changed = false;

    for (name, profile) in builtin_profiles() {
        if let std::collections::btree_map::Entry::Vacant(e) = pf.profiles.entry(name) {
            e.insert(profile);
            changed = true;
        }
    }

    if changed {
        if let Err(e) = save_file(&path, &pf) {
            eprintln!("warning: could not update profile config: {}", e);
        }
    }

    Ok(pf)
}

fn write_initial_profiles(path: &Path) -> std::result::Result<(), anyhow::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut pf = ProfileFile::default();
    for (name, profile) in builtin_profiles() {
        pf.profiles.insert(name, profile);
    }
    save_file(path, &pf)?;
    Ok(())
}

fn load_file(path: &Path) -> Result<ProfileFile> {
    let text = fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read {}: {}", path.display(), e))?;
    let pf: ProfileFile =
        toml::from_str(&text).map_err(|e| anyhow::anyhow!("invalid profiles.toml: {}", e))?;
    Ok(pf)
}

fn save_file(path: &Path, pf: &ProfileFile) -> std::result::Result<(), anyhow::Error> {
    let text = toml::to_string(pf)?;
    fs::write(path, text)?;
    Ok(())
}

/// Rewrite built-in profiles; prompt user before applying changes.
/// `confirm` receives the prompt message and returns true to proceed.
pub fn recreate(mut confirm: impl FnMut(&str) -> bool) -> Result<()> {
    let path = config_path()?;

    let mut pf = if path.exists() {
        load_file(&path)?
    } else {
        ProfileFile::default()
    };

    let builtins = builtin_profiles();
    let builtin_name_set: std::collections::HashSet<_> =
        builtins.iter().map(|(n, _)| n.as_str()).collect();

    let custom_names: Vec<_> = pf
        .profiles
        .keys()
        .filter(|k| !builtin_name_set.contains(k.as_str()))
        .cloned()
        .collect();

    let mut to_change: Vec<String> = Vec::new();
    let mut unchanged: Vec<String> = Vec::new();

    for (name, profile) in &builtins {
        match pf.profiles.get(name) {
            Some(existing) if existing == profile => unchanged.push(name.clone()),
            _ => to_change.push(name.clone()),
        }
    }

    if to_change.is_empty() {
        eprintln!("No changes needed");
        return Ok(());
    }

    eprintln!(
        "Will update: {}",
        if to_change.is_empty() {
            "none".into()
        } else {
            to_change.join(", ")
        }
    );
    eprintln!(
        "Unchanged: {}",
        if unchanged.is_empty() {
            "none".into()
        } else {
            unchanged.join(", ")
        }
    );
    eprintln!(
        "Ignored (custom): {}",
        if custom_names.is_empty() {
            "none".into()
        } else {
            custom_names.join(", ")
        }
    );

    if !confirm("Proceed? [y/N] ") {
        return Ok(());
    }

    for (name, profile) in &builtins {
        pf.profiles.insert(name.clone(), profile.clone());
    }

    match save_file(&path, &pf) {
        Ok(()) => {}
        Err(e) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).ok();
            }
            save_file(&path, &pf)
                .map_err(|_| anyhow::anyhow!("cannot write {}: {}", path.display(), e))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    static CONFIG_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_config_dir<F: FnOnce(&Path) -> R, R>(f: F) -> R {
        let _guard = CONFIG_TEST_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        std::env::set_var("CTOKEN_CONFIG_DIR", dir.path());
        let result = f(dir.path());
        std::env::remove_var("CTOKEN_CONFIG_DIR");
        result
    }

    #[test]
    fn first_run_creates_file() {
        with_config_dir(|dir| {
            let pf = load_or_init().unwrap();
            assert!(pf.profiles.contains_key("rust"));
            assert!(pf.profiles.contains_key("python"));
            let path = dir.join("profiles.toml");
            assert!(path.exists());
        });
    }

    #[test]
    fn append_missing_builtins() {
        with_config_dir(|dir| {
            // Write file with only "go" profile
            let path = dir.join("profiles.toml");
            let partial = "[go]\ninclude = [\"**/*.go\"]\n";
            fs::write(&path, partial).unwrap();

            let pf = load_or_init().unwrap();
            assert!(pf.profiles.contains_key("rust")); // added
            assert!(pf.profiles.contains_key("go")); // kept
        });
    }

    #[test]
    fn custom_profiles_preserved() {
        with_config_dir(|dir| {
            let path = dir.join("profiles.toml");
            let content = "[my_custom]\ninclude = [\"**/*.xyz\"]\n";
            fs::write(&path, content).unwrap();

            let pf = load_or_init().unwrap();
            assert!(pf.profiles.contains_key("my_custom"));
            assert!(pf.profiles.contains_key("rust"));
        });
    }

    #[test]
    fn roundtrip_serialization() {
        let mut pf = ProfileFile::default();
        for (name, profile) in builtin_profiles() {
            pf.profiles.insert(name, profile);
        }
        let text = toml::to_string(&pf).unwrap();
        let pf2: ProfileFile = toml::from_str(&text).unwrap();
        assert_eq!(pf.profiles, pf2.profiles);
    }

    #[test]
    fn recreate_no_changes_needed() {
        with_config_dir(|dir| {
            let path = dir.join("profiles.toml");
            // write exact builtins
            let mut pf = ProfileFile::default();
            for (name, profile) in builtin_profiles() {
                pf.profiles.insert(name, profile);
            }
            fs::write(&path, toml::to_string(&pf).unwrap()).unwrap();

            let mut prompted = false;
            recreate(|_| {
                prompted = true;
                true
            })
            .unwrap();
            assert!(!prompted, "should not prompt when nothing to change");
        });
    }

    #[test]
    fn recreate_partitioning() {
        with_config_dir(|dir| {
            let path = dir.join("profiles.toml");
            // write modified rust profile + custom entry
            let content =
                "[rust]\ninclude = [\"**/*.rs\"]\n[my_custom]\ninclude = [\"**/*.xyz\"]\n";
            fs::write(&path, content).unwrap();

            let mut confirmed = false;
            recreate(|_msg| {
                confirmed = true;
                true
            })
            .unwrap();
            assert!(confirmed);

            // built-in rust should be restored
            let pf = load_file(&path).unwrap();
            let rust = pf.profiles.get("rust").unwrap();
            assert!(rust.include.contains(&"Cargo.toml".to_string()));
            // custom preserved
            assert!(pf.profiles.contains_key("my_custom"));
        });
    }
}
