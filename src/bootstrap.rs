use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{scaffold, spec::ProjectSpec};

const ENGINE_FILES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "src/main.rs",
    "src/recipe.rs",
    "src/scaffold.rs",
    "src/spec.rs",
    "src/git.rs",
    "src/doctor.rs",
    "src/bootstrap.rs",
    ".github/workflows/ci.yml",
    ".stackpilot-template",
];

pub fn run(
    spec: &ProjectSpec,
    recipe_name: &str,
    recipes_dir: &Path,
    keep_engine: bool,
    force: bool,
) -> Result<()> {
    let root = env::current_dir().context("failed to determine current repository directory")?;

    if !root.join(".stackpilot-template").exists() && !force {
        bail!(
            "this repository is not marked as a StackPilot template; use --force to bootstrap anyway"
        );
    }

    let generated = scaffold::render_in_place(spec, recipe_name, recipes_dir, &root)?;

    if !keep_engine {
        cleanup_engine(&root, &generated)?;
    }

    println!("Bootstrapped {} in {}", spec.name, root.display());
    if keep_engine {
        println!("StackPilot engine files were preserved (--keep-engine)");
    } else {
        println!("StackPilot template engine files were removed");
    }

    Ok(())
}

fn cleanup_engine(root: &Path, generated: &[PathBuf]) -> Result<()> {
    let generated: HashSet<&Path> = generated.iter().map(PathBuf::as_path).collect();

    for relative in ENGINE_FILES {
        let relative_path = Path::new(relative);
        if generated.contains(relative_path) {
            continue;
        }

        let path = root.join(relative_path);
        if path.is_file() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove template file {}", path.display()))?;
        }
    }

    if !generated.iter().any(|path| path.starts_with("recipes")) {
        let recipes = root.join("recipes");
        if recipes.exists() {
            fs::remove_dir_all(&recipes).with_context(|| {
                format!("failed to remove template recipes {}", recipes.display())
            })?;
        }
    }

    remove_if_empty(&root.join("src"))?;

    Ok(())
}

fn remove_if_empty(directory: &Path) -> Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }

    if fs::read_dir(directory)?.next().is_none() {
        fs::remove_dir(directory)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::remove_if_empty;

    #[test]
    fn removes_empty_directories_only() {
        let root = tempdir().expect("temp directory");
        let empty = root.path().join("empty");
        fs::create_dir(&empty).expect("create empty directory");
        remove_if_empty(&empty).expect("remove empty directory");
        assert!(!empty.exists());

        let populated = root.path().join("populated");
        fs::create_dir(&populated).expect("create populated directory");
        fs::write(populated.join("file.txt"), "keep").expect("write file");
        remove_if_empty(&populated).expect("preserve populated directory");
        assert!(populated.exists());
    }
}
