use std::{fs, path::{Path, PathBuf}};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub files: Vec<RecipeFile>,
}

#[derive(Debug, Deserialize)]
pub struct RecipeFile {
    pub template: String,
    pub destination: String,
}

impl Recipe {
    pub fn load(recipes_dir: &Path, recipe_name: &str) -> Result<(Self, PathBuf)> {
        let recipe_dir = recipes_dir.join(recipe_name);
        let manifest_path = recipe_dir.join("recipe.toml");
        let raw = fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read recipe manifest {}", manifest_path.display()))?;

        let recipe: Recipe = toml::from_str(&raw)
            .with_context(|| format!("failed to parse recipe manifest {}", manifest_path.display()))?;

        Ok((recipe, recipe_dir))
    }
}
