# Repository inspection

> Development status: `stackpilot inspect` is part of the V0.2 work on `main` and is not included in the current v0.1.2 binary release yet.

StackPilot's repository intelligence starts with a deterministic inspection pass. It reads repository structure and small text configuration/source files, reports what it can prove, and highlights production-foundation gaps without modifying the repository.

## Usage

Inspect the current directory:

```bash
stackpilot inspect
```

Inspect another repository:

```bash
stackpilot inspect ../payments-api
```

## Current detection

The first inspection milestone detects:

- language markers for Rust, Go, TypeScript/JavaScript, Python, Java and C#;
- recognized frameworks including StackPilot's six golden paths plus several common alternatives;
- Dockerfile and Compose configuration;
- GitHub Actions, GitLab CI, Jenkins, Azure Pipelines and Bitbucket Pipelines;
- Terraform configuration;
- common health/readiness endpoint conventions;
- environment example and `.env` ignore hygiene;
- `.stackpilot.toml` project metadata.

Dependency/build directories such as `node_modules`, `target`, `dist`, `build`, virtual environments and `.git` are skipped. Symlinked directories/files are not followed. Text inspection is bounded by file size and directory depth so `inspect` remains predictable on normal repositories.

## Example shape

```text
StackPilot inspect
Repository: /workspace/payments-api
Languages: TypeScript
Frameworks: NestJS

Runtime
✓ Language — TypeScript
✓ Framework — NestJS
✓ Docker — Dockerfile and Compose detected (Dockerfile, compose.yaml)
✓ Health check — Health/readiness endpoint convention detected

Delivery
✓ CI/CD — GitHub Actions

Infrastructure
✓ Terraform — 3 Terraform file(s) detected

Configuration
✓ Environment config — Safe example detected and .env is ignored (.env.example)
✓ StackPilot metadata — .stackpilot.toml detected

Production gaps: none detected by the current inspection rules

Readiness scoring is not enabled yet; this report is deterministic and informational.
```

## Environment-file behavior

An ignored local `.env` file is normal developer behavior and is not treated as a failure. StackPilot warns when `.env` lacks detected ignore protection, when an example file exists without ignore protection, or when the repository has no safe environment example. `inspect` does not claim that a local file is Git-tracked unless StackPilot can prove that separately.

## Current non-goals

This milestone does not assign a readiness score, modify files, infer cloud architecture from source behavior, or use AI/LLMs to judge repositories. Those capabilities are separate roadmap stages. The next milestone will define an explainable readiness score on top of these deterministic findings; automated remediation will come after that.
