# Repository inspection

> Development status: `stackpilot inspect` and readiness scoring are part of the V0.2 work on `main` and are not included in the current v0.1.2 binary release yet.

StackPilot's repository intelligence is deterministic and read-only. It inspects repository structure and bounded text configuration/source files, reports what it can prove, calculates an explainable production-readiness score, and highlights concrete gaps without modifying the repository.

## Usage

Inspect the current directory:

```bash
stackpilot inspect
```

Inspect another repository:

```bash
stackpilot inspect ../payments-api
```

Require a minimum readiness score in CI:

```bash
stackpilot inspect --fail-below 80
```

`--fail-below` accepts a score from 0 to 100. StackPilot prints the complete report and exits non-zero when the calculated score is below the required threshold.

## Readiness model

The initial scoring contract is versioned as `readiness-v1` and totals 100 points:

| Category | Controls | Points |
| --- | --- | ---: |
| Runtime | Language, framework, Docker/container setup | 25 |
| Delivery | CI/CD configuration | 20 |
| Infrastructure | Terraform/infrastructure-as-code foundation | 15 |
| Security | Environment configuration hygiene | 25 |
| Operability | Health/readiness convention | 15 |
| **Total** |  | **100** |

A passed control receives its full weight, a warning receives half credit using integer points, and a missing control receives zero. The model is intentionally explicit and deterministic: the same repository state produces the same score under the same model version.

`.stackpilot.toml` is still reported as useful project metadata, but it does not affect readiness-v1. Existing repositories are not penalized merely because they were not created by StackPilot.

## Current detection

The inspection pass detects:

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
StackPilot Readiness: 85/100
Model: readiness-v1
Repository: /workspace/payments-api
Languages: TypeScript
Frameworks: NestJS

Category scores
Runtime        25/25
Delivery       20/20
Infrastructure  0/15
Security       25/25
Operability    15/15

Controls
✓ [Runtime       ] Language 5/5 — TypeScript
✓ [Runtime       ] Framework 5/5 — NestJS
✓ [Runtime       ] Docker 15/15 — Dockerfile and Compose detected
✓ [Delivery      ] CI/CD 20/20 — GitHub Actions
✗ [Infrastructure] Terraform 0/15 — No Terraform configuration detected
✓ [Security      ] Environment config 25/25 — Safe example detected and .env is ignored
✓ [Operability   ] Health check 15/15 — Health/readiness endpoint convention detected

Additional findings
! StackPilot metadata — .stackpilot.toml not found

Recommendations: 2
  - Add infrastructure-as-code when the service owns deployable infrastructure.
  - Add StackPilot project metadata so future remediation and upgrades can track repository intent.
```

With a policy threshold:

```text
Required threshold: 90
✗ Repository readiness is below the required threshold.
```

The process exits non-zero in that case, making the command suitable as a CI quality gate.

## Environment-file behavior

An ignored local `.env` file is normal developer behavior and is not treated as a failure. StackPilot warns when `.env` lacks detected ignore protection, when an example file exists without ignore protection, or when the repository has no safe environment example. `inspect` does not claim that a local file is Git-tracked unless StackPilot can prove that separately.

## Model boundaries

`readiness-v1` is deliberately small. It scores only controls that the current deterministic inspection layer can support reliably. Security scanning, SBOMs, dependency update automation, image scanning and more advanced operability checks are separate roadmap stages; they can extend a future versioned readiness model without silently changing the meaning of `readiness-v1`.

`stackpilot inspect` still does not modify repository files, infer architecture from runtime behavior, or use AI/LLMs to judge repositories. Automated remediation comes after the readiness model.

## Optional gODtECH Steward integration

StackPilot can display a local gODtECH Steward scan alongside its own readiness result without changing `readiness-v1`:

```bash
stackpilot inspect . --steward-report steward-report.json
```

The Steward report must use the versioned `schemaVersion: 1` contract. StackPilot treats Steward as an optional observational source and does not execute Steward remediation or reimplement its generic housekeeping rules. See [`steward-integration.md`](./steward-integration.md) for the contract and privacy boundary.
