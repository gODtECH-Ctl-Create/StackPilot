mod doctor;
mod git;
mod recipe;
mod scaffold;
mod spec;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

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
