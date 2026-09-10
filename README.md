# StackPilot

StackPilot is a Rust-powered GitHub template and project scaffolding engine for creating consistent, production-minded repositories from reusable recipes.

## GitHub template flow

1. Click **Use this template** on GitHub.
2. Clone the new repository.
3. Run:

```bash
cargo run -- bootstrap
```

StackPilot will detect the repository name, ask for the project type, language, framework, database, cloud, container, CI, and Terraform choices, render the selected recipe directly into the repository, persist the choices in `.stackpilot.toml`, and remove the template-engine source files.

To test bootstrap without removing the engine:

```bash
cargo run -- bootstrap --keep-engine
```

## Create a separate project

Interactive:

```bash
cargo run -- new
```

Or:

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

## Current capabilities

- Native Rust CLI
- GitHub-template in-place bootstrap
- Interactive project specification
- Rust, Go, TypeScript, Python, Java, and C# stack selection
- Framework, database, cloud, Docker, CI, and Terraform choices
- TOML recipe manifests
- MiniJinja rendering
- Conditional recipe files
- Path-safe project naming
- Automatic Git initialization for generated projects
- Recipe discovery and diagnostics
- Persistent `.stackpilot.toml` project profile
- Transactional cleanup when project generation fails

## Architecture

The StackPilot engine is independent from generated project languages. Recipes own stack-specific output, while the Rust core handles selection, validation, rendering, repository operations, and lifecycle behavior.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```
