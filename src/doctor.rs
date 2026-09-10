use std::path::Path;

use anyhow::{Result, bail};

use crate::{git, recipe::Recipe};

pub fn run(recipes_dir: &Path) -> Result<()> {
    println!("StackPilot doctor");

    if git::is_available() {
        println!("✓ Git available");
    } else {
        println!("! Git not found; project generation requires --no-git");
    }

    let recipes = Recipe::discover(recipes_dir)?;
    if recipes.is_empty() {
        bail!("no valid recipes found in {}", recipes_dir.display());
    }

    println!("✓ {} recipe(s) validated", recipes.len());
    println!("✓ recipe directory: {}", recipes_dir.display());

    Ok(())
}
