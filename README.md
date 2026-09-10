# StackPilot

StackPilot is a Rust-powered GitHub template and project scaffolding engine for creating consistent, production-minded repositories from reusable recipes.

## Status

Early foundation. The first milestone is a local CLI that can render a project from a StackPilot recipe.

## Planned first command

```bash
stackpilot new my-service --recipe base
```

## Core design

- Rust native CLI
- TOML recipe manifests
- MiniJinja templates
- Deterministic file generation
- Language/framework-agnostic project output
- GitHub Actions validation

## Development

```bash
cargo build
cargo test
cargo run -- --help
```
