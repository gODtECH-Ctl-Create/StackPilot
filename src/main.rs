mod bootstrap;
mod doctor;
mod git;
mod inspect;
mod readiness;
mod recipe;
mod scaffold;
mod spec;

use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use clap::{ArgAction, Args, Parser, Subcommand};

use recipe::Recipe;
use spec::ProjectSpec;

const DEFAULT_RECIPES_DIR: &str = "recipes";

#[derive(Debug, Parser)]
#[command(
    name = "stackpilot",
    version,
    about = "Production-minded project scaffolding from reusable recipes"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Args)]
struct SpecOptions {
    /// Project type used with --non-interactive.
    #[arg(long, default_value = "Generic")]
    kind: String,

    /// Programming language used with --non-interactive.
    #[arg(long, default_value = "Generic")]
    language: String,

    /// Framework used with --non-interactive. Auto chooses StackPilot's golden path.
    #[arg(long, default_value = "Auto")]
    framework: String,

    /// Database used with --non-interactive.
    #[arg(long, default_value = "None")]
    database: String,

    /// Cloud used with --non-interactive.
    #[arg(long, default_value = "None")]
    cloud: String,

    /// Include container support with --non-interactive.
    #[arg(long, default_value_t = false, action = ArgAction::Set)]
    docker: bool,

    /// Include CI support with --non-interactive.
    #[arg(long, default_value_t = false, action = ArgAction::Set)]
    ci: bool,

    /// Include Terraform support with --non-interactive.
    #[arg(long, default_value_t = false, action = ArgAction::Set)]
    terraform: bool,
}

impl SpecOptions {
    fn into_project_spec(self, name: String) -> Result<ProjectSpec> {
        ProjectSpec::configured(
            name,
            self.kind,
            self.language,
            self.framework,
            self.database,
            self.cloud,
            self.docker,
            self.ci,
            self.terraform,
        )
    }
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a new project from a StackPilot recipe.
    New {
        /// Name of the project to generate. Omit it to be prompted.
        name: Option<String>,

        /// Recipe name found under the recipes directory.
        #[arg(short, long, default_value = "base")]
        recipe: String,

        /// Directory that contains StackPilot recipes.
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,

        /// Parent directory where the project will be created.
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// Disable interactive prompts and use values supplied by flags.
        #[arg(long)]
        non_interactive: bool,

        #[command(flatten)]
        spec: SpecOptions,

        /// Do not initialize a Git repository in the generated project.
        #[arg(long)]
        no_git: bool,
    },

    /// Preview the files that a recipe would generate without writing anything.
    Plan {
        /// Project name used while resolving template destinations.
        name: Option<String>,

        /// Recipe name found under the recipes directory.
        #[arg(short, long, default_value = "base")]
        recipe: String,

        /// Directory that contains StackPilot recipes.
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,

        /// Disable interactive prompts and use values supplied by flags.
        #[arg(long)]
        non_interactive: bool,

        #[command(flatten)]
        spec: SpecOptions,
    },

    /// Convert a repository created with "Use this template" into a project.
    Bootstrap {
        /// Project name. Defaults to the current repository directory name.
        #[arg(short, long)]
        name: Option<String>,

        /// Recipe name found under the recipes directory.
        #[arg(short, long, default_value = "base")]
        recipe: String,

        /// Directory that contains StackPilot recipes.
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,

        /// Disable interactive prompts and use values supplied by flags.
        #[arg(long)]
        non_interactive: bool,

        #[command(flatten)]
        spec: SpecOptions,

        /// Preserve the StackPilot engine files after bootstrapping.
        #[arg(long)]
        keep_engine: bool,

        /// Allow bootstrapping a repository without the template marker.
        #[arg(long)]
        force: bool,
    },

    /// Inspect an existing repository and calculate its production readiness score.
    Inspect {
        /// Repository directory to inspect. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Exit non-zero when readiness is below this 0-100 threshold.
        #[arg(long, value_name = "SCORE")]
        fail_below: Option<u8>,
    },

    /// List available StackPilot recipes.
    Recipes {
        /// Directory that contains StackPilot recipes.
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,
    },

    /// Validate the local StackPilot environment and recipes.
    Doctor {
        /// Directory that contains StackPilot recipes.
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            name,
            recipe,
            recipes_dir,
            output,
            non_interactive,
            spec,
            no_git,
        } => {
            let recipes_dir = resolve_recipes_dir(recipes_dir);
            let project_spec = resolve_spec(name, non_interactive, spec)?;
            let destination =
                scaffold::create_project(&project_spec, &recipe, &recipes_dir, &output)?;

            if !no_git {
                git::init_repository(&destination)?;
            }

            println!("Created {} at {}", project_spec.name, destination.display());
            println!("Recipe: {recipe}");
            println!(
                "Stack: {} / {} / {}",
                project_spec.language, project_spec.framework, project_spec.database
            );
        }
        Commands::Plan {
            name,
            recipe,
            recipes_dir,
            non_interactive,
            spec,
        } => {
            let recipes_dir = resolve_recipes_dir(recipes_dir);
            let project_spec = resolve_spec(name, non_interactive, spec)?;
            let files = scaffold::plan_project(&project_spec, &recipe, &recipes_dir)?;

            println!("StackPilot plan for {} ({recipe})", project_spec.name);
            for path in files {
                println!("  + {}", path.display());
            }
        }
        Commands::Bootstrap {
            name,
            recipe,
            recipes_dir,
            non_interactive,
            spec,
            keep_engine,
            force,
        } => {
            let recipes_dir = resolve_recipes_dir(recipes_dir);
            let name = name.unwrap_or(current_repository_name()?);
            let project_spec = if non_interactive {
                spec.into_project_spec(name)?
            } else {
                ProjectSpec::interactive(Some(name))?
            };

            bootstrap::run(&project_spec, &recipe, &recipes_dir, keep_engine, force)?;
        }
        Commands::Inspect { path, fail_below } => {
            readiness::run(&path, fail_below)?;
        }
        Commands::Recipes { recipes_dir } => {
            let recipes_dir = resolve_recipes_dir(recipes_dir);
            let recipes = Recipe::discover(&recipes_dir)?;
            if recipes.is_empty() {
                println!("No recipes found in {}", recipes_dir.display());
            } else {
                for recipe in recipes {
                    println!("{} - {}", recipe.name, recipe.description);
                }
            }
        }
        Commands::Doctor { recipes_dir } => {
            let recipes_dir = resolve_recipes_dir(recipes_dir);
            doctor::run(&recipes_dir)?;
        }
    }

    Ok(())
}

fn resolve_recipes_dir(recipes_dir: PathBuf) -> PathBuf {
    let local_exists = recipes_dir.is_dir();
    resolve_recipes_dir_with_executable(recipes_dir, local_exists, env::current_exe().ok())
}

fn resolve_recipes_dir_with_executable(
    recipes_dir: PathBuf,
    local_exists: bool,
    executable: Option<PathBuf>,
) -> PathBuf {
    if recipes_dir.as_path() != std::path::Path::new(DEFAULT_RECIPES_DIR) || local_exists {
        return recipes_dir;
    }

    if let Some(executable) = executable
        && let Some(parent) = executable.parent()
    {
        let installed_recipes = parent.join(DEFAULT_RECIPES_DIR);
        if installed_recipes.is_dir() {
            return installed_recipes;
        }
    }

    recipes_dir
}

fn resolve_spec(
    name: Option<String>,
    non_interactive: bool,
    spec: SpecOptions,
) -> Result<ProjectSpec> {
    if non_interactive {
        let name = name.context("--non-interactive requires a project name")?;
        spec.into_project_spec(name)
    } else {
        ProjectSpec::interactive(name)
    }
}

fn current_repository_name() -> Result<String> {
    let current = env::current_dir().context("failed to determine current directory")?;
    current
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .context("current directory does not have a valid UTF-8 name")
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::tempdir;

    use super::resolve_recipes_dir_with_executable;

    #[test]
    fn falls_back_to_recipes_beside_installed_executable() {
        let install = tempdir().expect("install directory");
        let recipes = install.path().join("recipes");
        fs::create_dir(&recipes).expect("create recipes directory");
        let executable = install.path().join("stackpilot.exe");

        let resolved =
            resolve_recipes_dir_with_executable(PathBuf::from("recipes"), false, Some(executable));

        assert_eq!(resolved, recipes);
    }

    #[test]
    fn preserves_explicit_recipe_directory() {
        let custom = PathBuf::from("custom-recipes");
        let resolved = resolve_recipes_dir_with_executable(custom.clone(), false, None);
        assert_eq!(resolved, custom);
    }
}
