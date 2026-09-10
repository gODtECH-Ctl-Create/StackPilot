use std::{
    fs,
    path::{Path, PathBuf},
};

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
    #[serde(default)]
    pub when: Option<String>,
}

#[derive(Debug)]
pub struct RecipeSummary {
    pub name: String,
    pub description: String,
}

impl Recipe {
    pub fn load(recipes_dir: &Path, recipe_name: &str) -> Result<(Self, PathBuf)> {
        let recipe_dir = recipes_dir.join(recipe_name);
        let manifest_path = recipe_dir.join("recipe.toml");
        let raw = fs::read_to_string(&manifest_path).with_context(|| {
            format!(
                "failed to read recipe manifest {}",
                manifest_path.display()
            )
        })?;

        let recipe: Recipe = toml::from_str(&raw).with_context(|| {
            format!(
                "failed to parse recipe manifest {}",
                manifest_path.display()
            )
        })?;

        Ok((recipe, recipe_dir))
    }

    pub fn discover(recipes_dir: &Path) -> Result<Vec<RecipeSummary>> {
        let entries = fs::read_dir(recipes_dir)
            .with_context(|| format!("failed to read recipes directory {}", recipes_dir.display()))?;

        let mut recipes = Vec::new();

        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let recipe_name = entry.file_name().to_string_lossy().to_string();
            if !entry.path().join("recipe.toml").exists() {
                continue;
            }

            let (recipe, _) = Self::load(recipes_dir, &recipe_name)?;
            recipes.push(RecipeSummary {
                name: recipe.name,
                description: recipe.description.unwrap_or_else(|| "No description".to_string()),
            });
        }

        recipes.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(recipes)
    }
}
