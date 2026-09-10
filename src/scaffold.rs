use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use minijinja::{Environment, context};

use crate::{recipe::Recipe, spec::ProjectSpec};

pub fn create_project(
    spec: &ProjectSpec,
    recipe_name: &str,
    recipes_dir: &Path,
    output_dir: &Path,
) -> Result<PathBuf> {
    let destination = output_dir.join(&spec.name);
    if destination.exists() {
        bail!("destination already exists: {}", destination.display());
    }

    fs::create_dir_all(&destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;

    if let Err(error) = render_in_place(spec, recipe_name, recipes_dir, &destination) {
        let _ = fs::remove_dir_all(&destination);
        return Err(error);
    }

    Ok(destination)
}

pub fn plan_project(
    spec: &ProjectSpec,
    recipe_name: &str,
    recipes_dir: &Path,
) -> Result<Vec<PathBuf>> {
    let (recipe, _) = Recipe::load(recipes_dir, recipe_name)?;
    selected_destinations(spec, &recipe)
}

pub fn render_in_place(
    spec: &ProjectSpec,
    recipe_name: &str,
    recipes_dir: &Path,
    destination: &Path,
) -> Result<Vec<PathBuf>> {
    let (recipe, recipe_dir) = Recipe::load(recipes_dir, recipe_name)?;
    render_files(spec, &recipe, &recipe_dir, destination)
}

pub fn render_destination(
    spec: &ProjectSpec,
    recipe_name: &str,
    recipes_dir: &Path,
    destination: &Path,
) -> Result<String> {
    let (recipe, recipe_dir) = Recipe::load(recipes_dir, recipe_name)?;
    let selected = selected_files(spec, &recipe)?;
    let (file, _) = selected
        .into_iter()
        .find(|(_, selected_destination)| selected_destination == destination)
        .with_context(|| {
            format!(
                "recipe '{}' does not select {} for the configured project",
                recipe.name,
                destination.display()
            )
        })?;

    render_file(spec, &recipe, &recipe_dir, file)
}

fn render_files(
    spec: &ProjectSpec,
    recipe: &Recipe,
    recipe_dir: &Path,
    destination: &Path,
) -> Result<Vec<PathBuf>> {
    let selected = selected_files(spec, recipe)?;
    let mut generated = Vec::with_capacity(selected.len());

    for (file, relative_destination) in selected {
        let rendered = render_file(spec, recipe, recipe_dir, file)?;
        let target = destination.join(&relative_destination);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, rendered)
            .with_context(|| format!("failed to write {}", target.display()))?;
        generated.push(relative_destination);
    }

    Ok(generated)
}

fn render_file(
    spec: &ProjectSpec,
    recipe: &Recipe,
    recipe_dir: &Path,
    file: &crate::recipe::RecipeFile,
) -> Result<String> {
    let source = recipe_dir.join("templates").join(&file.template);
    let template_source = fs::read_to_string(&source)
        .with_context(|| format!("failed to read template {}", source.display()))?;

    let mut env = Environment::new();
    env.add_template_owned(file.template.clone(), template_source)
        .with_context(|| format!("failed to load template {}", file.template))?;

    env.get_template(&file.template)?
        .render(context! {
            project_name => spec.name.as_str(),
            project_kind => spec.kind.as_str(),
            language => spec.language.as_str(),
            framework => spec.framework.as_str(),
            database => spec.database.as_str(),
            cloud => spec.cloud.as_str(),
            docker => spec.docker,
            ci => spec.ci,
            terraform => spec.terraform,
            recipe_name => recipe.name.as_str(),
            recipe_description => recipe.description.clone().unwrap_or_default(),
        })
        .with_context(|| format!("failed to render template {}", file.template))
}

fn selected_destinations(spec: &ProjectSpec, recipe: &Recipe) -> Result<Vec<PathBuf>> {
    Ok(selected_files(spec, recipe)?
        .into_iter()
        .map(|(_, destination)| destination)
        .collect())
}

fn selected_files<'a>(
    spec: &ProjectSpec,
    recipe: &'a Recipe,
) -> Result<Vec<(&'a crate::recipe::RecipeFile, PathBuf)>> {
    let mut selected = Vec::new();
    let mut destinations = HashSet::new();

    for file in &recipe.files {
        if let Some(condition) = &file.when
            && !matches_condition(condition, spec)?
        {
            continue;
        }

        let destination = safe_destination(&file.destination, &spec.name)?;
        if !destinations.insert(destination.clone()) {
            bail!(
                "recipe '{}' resolves more than one file to {}",
                recipe.name,
                destination.display()
            );
        }
        selected.push((file, destination));
    }

    Ok(selected)
}

fn safe_destination(destination: &str, project_name: &str) -> Result<PathBuf> {
    let rendered = destination.replace("{{project_name}}", project_name);
    let path = PathBuf::from(&rendered);

    if path.as_os_str().is_empty() || path.is_absolute() {
        bail!("recipe destination must be a non-empty relative path: {rendered}");
    }

    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        bail!("recipe destination escapes the project directory: {rendered}");
    }

    Ok(path)
}

fn matches_condition(condition: &str, spec: &ProjectSpec) -> Result<bool> {
    for clause in condition.split("&&") {
        if !matches_clause(clause.trim(), spec)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn matches_clause(clause: &str, spec: &ProjectSpec) -> Result<bool> {
    match clause {
        "docker" => Ok(spec.docker),
        "ci" => Ok(spec.ci),
        "terraform" => Ok(spec.terraform),
        "!docker" => Ok(!spec.docker),
        "!ci" => Ok(!spec.ci),
        "!terraform" => Ok(!spec.terraform),
        _ => {
            let Some((key, expected)) = clause.split_once('=') else {
                bail!("unsupported recipe condition: {clause}");
            };

            let actual = match key.trim() {
                "kind" => &spec.kind,
                "language" => &spec.language,
                "framework" => &spec.framework,
                "database" => &spec.database,
                "cloud" => &spec.cloud,
                _ => bail!("unsupported recipe condition key: {}", key.trim()),
            };

            Ok(actual.eq_ignore_ascii_case(expected.trim()))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{matches_condition, render_destination, safe_destination};
    use crate::spec::ProjectSpec;

    #[test]
    fn renders_project_name_in_safe_destination() {
        assert_eq!(
            safe_destination("services/{{project_name}}/README.md", "payments")
                .expect("safe destination"),
            std::path::PathBuf::from("services/payments/README.md")
        );
    }

    #[test]
    fn rejects_destinations_that_escape_project_root() {
        assert!(safe_destination("../outside.txt", "payments").is_err());
        assert!(safe_destination("/tmp/outside.txt", "payments").is_err());
    }

    #[test]
    fn evaluates_boolean_and_value_conditions() {
        let spec = ProjectSpec::configured(
            "payments".to_string(),
            "Generic".to_string(),
            "Generic".to_string(),
            "None".to_string(),
            "None".to_string(),
            "None".to_string(),
            false,
            false,
            false,
        )
        .expect("valid project spec");
        assert!(!matches_condition("docker", &spec).expect("condition should evaluate"));
        assert!(matches_condition("language=Generic", &spec).expect("condition should evaluate"));
    }

    #[test]
    fn evaluates_compound_conditions() {
        let spec = ProjectSpec::configured(
            "payments".to_string(),
            "Backend API".to_string(),
            "Rust".to_string(),
            "Axum".to_string(),
            "PostgreSQL".to_string(),
            "AWS".to_string(),
            true,
            true,
            true,
        )
        .expect("valid project spec");

        assert!(
            matches_condition("language=Rust && framework=Axum && docker", &spec)
                .expect("condition should evaluate")
        );
        assert!(
            !matches_condition("language=Rust && framework=Actix Web", &spec)
                .expect("condition should evaluate")
        );
    }

    #[test]
    fn renders_one_selected_recipe_destination() {
        let spec = ProjectSpec::configured(
            "payments".to_string(),
            "Backend API".to_string(),
            "Go".to_string(),
            "Chi".to_string(),
            "None".to_string(),
            "None".to_string(),
            true,
            false,
            false,
        )
        .expect("valid spec");

        let rendered = render_destination(
            &spec,
            "base",
            Path::new("recipes"),
            Path::new("Dockerfile"),
        )
        .expect("render Dockerfile");

        assert!(rendered.contains("FROM golang:1.27-alpine AS build"));
        assert!(rendered.contains("ENTRYPOINT [\"/app\"]"));
    }
}
