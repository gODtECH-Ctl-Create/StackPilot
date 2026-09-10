# StackPilot

StackPilot is a Rust-powered GitHub template and project scaffolding engine for creating consistent, production-minded repositories from reusable recipes.

## Current capabilities

- Interactive project setup
- Language/framework/database/cloud selection
- TOML recipe manifests
- MiniJinja rendering
- Conditional recipe files
- Safe project-name validation
- Automatic Git initialization
- Recipe discovery
- Environment/recipe diagnostics
- Language-agnostic generated repository metadata

## Create a project

Interactive:

```bash
cargo run -- new
```

Or provide the name directly:

```bash
cargo run -- new payment-service --recipe base
```

For automation:

```bash
cargo run -- new payment-service --recipe base --non-interactive --no-git
```

## Other commands

```bash
cargo run -- recipes
cargo run -- doctor
```

## Architecture

The Rust CLI is deliberately independent from generated project languages. Recipes can target Rust, Go, TypeScript, Python, Java, C#, or additional stacks without changing the core engine.

Generated repositories persist their selected project profile in `.stackpilot.toml`, allowing future StackPilot operations to understand and evolve the project.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```
