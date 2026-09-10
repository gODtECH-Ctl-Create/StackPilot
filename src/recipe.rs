use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail};
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
        validate_recipe_name(recipe_name)?;

        let recipe_dir = recipes_dir.join(recipe_name);
        let manifest_path = recipe_dir.join("recipe.toml");
        let raw = fs::read_to_string(&manifest_path).with_context(|| {
            format!("failed to read recipe manifest {}", manifest_path.display())
        })?;

        let recipe: Recipe = toml::from_str(&raw).with_context(|| {
            format!(
                "failed to parse recipe manifest {}",
                manifest_path.display()
            )
        })?;

        recipe.validate(&recipe_dir)?;
        Ok((recipe, recipe_dir))
    }

    pub fn discover(recipes_dir: &Path) -> Result<Vec<RecipeSummary>> {
        let entries = fs::read_dir(recipes_dir).with_context(|| {
            format!("failed to read recipes directory {}", recipes_dir.display())
        })?;

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
                description: recipe
                    .description
                    .unwrap_or_else(|| "No description".to_string()),
            });
        }

        recipes.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(recipes)
    }

    fn validate(&self, recipe_dir: &Path) -> Result<()> {
        if self.name.trim().is_empty() {
            bail!("recipe name cannot be empty");
        }
        if self.files.is_empty() {
            bail!("recipe '{}' does not define any files", self.name);
        }

        for file in &self.files {
            validate_relative_path(&file.template, "template")?;
            validate_relative_path(&file.destination, "destination")?;

            if file
                .when
                .as_ref()
                .is_some_and(|condition| condition.trim().is_empty())
            {
                bail!("recipe '{}' contains an empty condition", self.name);
            }

            let template = recipe_dir.join("templates").join(&file.template);
            if !template.is_file() {
                bail!(
                    "recipe '{}' references missing template {}",
                    self.name,
                    template.display()
                );
            }
        }

        Ok(())
    }
}

fn validate_recipe_name(name: &str) -> Result<()> {
    let path = Path::new(name);
    let mut components = path.components();
    let first = components.next();

    if name.trim().is_empty()
        || !matches!(first, Some(Component::Normal(_)))
        || components.next().is_some()
    {
        bail!("recipe name must be a single safe directory name");
    }

    Ok(())
}

fn validate_relative_path(value: &str, label: &str) -> Result<()> {
    let path = Path::new(value);
    if value.trim().is_empty() || path.is_absolute() {
        bail!("recipe {label} must be a non-empty relative path: {value}");
    }

    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        bail!("recipe {label} escapes its allowed directory: {value}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_recipe_name, validate_relative_path};

    #[test]
    fn rejects_recipe_path_traversal() {
        assert!(validate_recipe_name("../outside").is_err());
        assert!(validate_recipe_name("nested/recipe").is_err());
        assert!(validate_recipe_name("base").is_ok());
    }

    #[test]
    fn rejects_template_path_traversal() {
        assert!(validate_relative_path("../../secret", "template").is_err());
        assert!(validate_relative_path("rust/axum/main.rs.j2", "template").is_ok());
    }
}
