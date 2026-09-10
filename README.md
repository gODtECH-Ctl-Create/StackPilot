# StackPilot

StackPilot is a Rust-powered GitHub template and project scaffolding engine for creating consistent, production-minded repositories from a small set of deliberately opinionated golden paths.

## Product philosophy

StackPilot is not a framework picker with hundreds of combinations. It chooses a production default for each supported ecosystem and keeps the generated repositories structurally consistent.

| Language | StackPilot golden path |
| --- | --- |
| Rust | Axum |
| Go | Chi |
| TypeScript | NestJS |
| Python | FastAPI |
| Java | Spring Boot |
| C# | ASP.NET Core |

For deployable services, StackPilot defaults toward PostgreSQL, AWS, Docker, CI, and Terraform. Advanced automation can still override supported infrastructure choices, but the normal workflow recommends rather than overwhelms.

Every generated service follows the same initial contract: a `/health` endpoint, port `3000`, container support, language-native CI, environment metadata, and optional Terraform infrastructure.

> **v0.1 focus:** the fully validated golden paths are backend services/APIs. Additional project shapes such as frontend, full-stack, workers, CLI apps, and libraries remain roadmap targets rather than pretending to be production-ready today.

## Install

Tagged releases publish native StackPilot binaries for Linux x86_64, macOS Intel, macOS Apple Silicon, and Windows x86_64.

After the first release is published, Linux or macOS users can install the latest version with:

```bash
curl -fsSL https://raw.githubusercontent.com/gODtECH-Ctl-Create/StackPilot/main/scripts/install.sh | sh
```

Windows PowerShell:

```powershell
irm https://raw.githubusercontent.com/gODtECH-Ctl-Create/StackPilot/main/scripts/install.ps1 | iex
```

Until then, contributors can run StackPilot from source with `cargo run --` as shown below.

## GitHub template flow

The lowest-friction path is GitHub Actions:

1. Click **Use this template** on GitHub.
2. Open **Actions → Configure StackPilot Template** in the new repository.
3. Choose the language and adjust infrastructure only when necessary.
4. Run the workflow. StackPilot previews the plan, generates the repository, and commits the result.

For local interactive setup after cloning:

```bash
stackpilot bootstrap
```

When working from source:

```bash
cargo run -- bootstrap
```

StackPilot detects the repository name, recommends the golden framework for the selected language, asks only for the remaining deployment decisions, renders the selected recipe directly into the repository, persists the choices in `.stackpilot.toml`, and removes the template-engine source files.

To test bootstrap without removing the engine:

```bash
stackpilot bootstrap --keep-engine
```

## Create a separate project

Interactive:

```bash
stackpilot new
```

Or create a named project:

```bash
stackpilot new payment-service --recipe base
```

For automation, `--framework Auto` resolves to the StackPilot golden path:

```bash
stackpilot new payment-service \
  --recipe base \
  --non-interactive \
  --kind "Backend API" \
  --language Go \
  --framework Auto \
  --database PostgreSQL \
  --cloud AWS \
  --docker true \
  --ci true \
  --terraform true
```

Preview exactly what will be generated without writing files:

```bash
stackpilot plan payment-service \
  --non-interactive \
  --kind "Backend API" \
  --language Go \
  --framework Auto \
  --database PostgreSQL \
  --cloud AWS \
  --docker true \
  --ci true \
  --terraform true
```

## Other commands

```bash
stackpilot recipes
stackpilot doctor
```

## Current capabilities

- Native Rust CLI
- GitHub-template in-place bootstrap
- GitHub Actions setup UI
- Opinionated framework selection with `Auto`
- Rust/Axum, Go/Chi, TypeScript/NestJS, Python/FastAPI, Java/Spring Boot, and C#/ASP.NET Core backend golden paths
- Shared production container and Compose policy
- Language-native generated CI
- PostgreSQL-first database profile with MySQL, MongoDB, SQLite, or no database as supported alternatives
- AWS-first cloud profile with Azure, GCP, or no cloud as supported alternatives
- Terraform foundation
- TOML recipe manifests and MiniJinja rendering
- Compound conditional recipe files
- Non-destructive scaffold planning
- Path-safe project and recipe handling
- Automatic Git initialization for generated projects
- Recipe discovery and diagnostics
- Persistent `.stackpilot.toml` project profile
- Transactional cleanup when project generation fails
- CI compatibility matrix that generates and builds every golden path
- Cross-platform tagged binary releases and one-command installers

## Architecture

The StackPilot engine is independent from generated project languages. Recipes own stack-specific output, while the Rust core handles opinionated selection, validation, rendering, repository operations, safety, and lifecycle behavior.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```
