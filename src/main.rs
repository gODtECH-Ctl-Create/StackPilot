mod bootstrap;
mod doctor;
mod git;
mod recipe;
mod scaffold;
mod spec;

use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use clap::{ArgAction, Parser, Subcommand};

use recipe::Recipe;
use spec::ProjectSpec;

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

        /// Disable interactive prompts and use safe generic defaults.
        #[arg(long)]
        non_interactive: bool,

        /// Do not initialize a Git repository in the generated project.
        #[arg(long)]
        no_git: bool,
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

        /// Project type used with --non-interactive.
        #[arg(long, default_value = "Generic")]
        kind: String,

        /// Programming language used with --non-interactive.
        #[arg(long, default_value = "Generic")]
        language: String,

        /// Framework used with --non-interactive.
        #[arg(long, default_value = "None")]
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

        /// Preserve the StackPilot engine files after bootstrapping.
        #[arg(long)]
        keep_engine: bool,

        /// Allow bootstrapping a repository without the template marker.
        #[arg(long)]
        force: bool,
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
            no_git,
        } => {
            let spec = if non_interactive {
                let name = name.context("--non-interactive requires a project name")?;
                ProjectSpec::minimal(name)?
            } else {
                ProjectSpec::interactive(name)?
            };

            let destination = scaffold::create_project(&spec, &recipe, &recipes_dir, &output)?;

            if !no_git {
                git::init_repository(&destination)?;
            }

            println!("Created {} at {}", spec.name, destination.display());
            println!("Recipe: {recipe}");
            println!(
                "Stack: {} / {} / {}",
                spec.language, spec.framework, spec.database
            );
        }
        Commands::Bootstrap {
            name,
            recipe,
            recipes_dir,
            non_interactive,
            kind,
            language,
            framework,
            database,
            cloud,
            docker,
            ci,
            terraform,
            keep_engine,
            force,
        } => {
            let name = name.unwrap_or(current_repository_name()?);
            let spec = if non_interactive {
                ProjectSpec::configured(
                    name, kind, language, framework, database, cloud, docker, ci, terraform,
                )?
            } else {
                ProjectSpec::interactive(Some(name))?
            };

            bootstrap::run(&spec, &recipe, &recipes_dir, keep_engine, force)?;
        }
        Commands::Recipes { recipes_dir } => {
            let recipes = Recipe::discover(&recipes_dir)?;
            if recipes.is_empty() {
                println!("No recipes found in {}", recipes_dir.display());
            } else {
                for recipe in recipes {
                    println!("{} - {}", recipe.name, recipe.description);
                }
            }
        }
        Commands::Doctor { recipes_dir } => doctor::run(&recipes_dir)?,
    }

    Ok(())
}

fn current_repository_name() -> Result<String> {
    let current = env::current_dir().context("failed to determine current directory")?;
    current
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .context("current directory does not have a valid UTF-8 name")
}
