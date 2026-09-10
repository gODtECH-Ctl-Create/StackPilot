mod recipe;
mod scaffold;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "stackpilot", version, about = "Production-minded project scaffolding from reusable recipes")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a new project from a StackPilot recipe.
    New {
        /// Name of the project to generate.
        name: String,

        /// Recipe name found under the recipes directory.
        #[arg(short, long, default_value = "base")]
        recipe: String,

        /// Directory that contains StackPilot recipes.
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,

        /// Parent directory where the project will be created.
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
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
        } => {
            let destination = scaffold::create_project(&name, &recipe, &recipes_dir, &output)?;
            println!("Created {} at {}", name, destination.display());
        }
    }

    Ok(())
}
