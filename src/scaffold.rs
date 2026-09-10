use std::{fs, path::{Path, PathBuf}};

use anyhow::{bail, Context, Result};
use minijinja::{context, Environment};

use crate::recipe::Recipe;

pub fn create_project(
    project_name: &str,
    recipe_name: &str,
    recipes_dir: &Path,
    output_dir: &Path,
) -> Result<PathBuf> {
    let destination = output_dir.join(project_name);
    if destination.exists() {
        bail!("destination already exists: {}", destination.display());
    }

    let (recipe, recipe_dir) = Recipe::load(recipes_dir, recipe_name)?;
    fs::create_dir_all(&destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;

    let mut env = Environment::new();

    for file in &recipe.files {
        let source = recipe_dir.join("templates").join(&file.template);
        let template_source = fs::read_to_string(&source)
            .with_context(|| format!("failed to read template {}", source.display()))?;

        env.add_template_owned(file.template.clone(), template_source)
            .with_context(|| format!("failed to load template {}", file.template))?;

        let rendered = env
            .get_template(&file.template)?
            .render(context! {
                project_name => project_name,
                recipe_name => recipe.name,
                recipe_description => recipe.description.clone().unwrap_or_default(),
            })?;

        let relative_destination = render_destination(&file.destination, project_name);
        let target = destination.join(relative_destination);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, rendered)
            .with_context(|| format!("failed to write {}", target.display()))?;
    }

    Ok(destination)
}

fn render_destination(destination: &str, project_name: &str) -> String {
    destination.replace("{{project_name}}", project_name)
}

#[cfg(test)]
mod tests {
    use super::render_destination;

    #[test]
    fn renders_project_name_in_destination() {
        assert_eq!(
            render_destination("services/{{project_name}}/README.md", "payments"),
            "services/payments/README.md"
        );
    }
}
