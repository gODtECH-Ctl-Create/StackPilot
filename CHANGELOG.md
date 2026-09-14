# Changelog

All notable StackPilot releases are summarized here. Release tags remain the authoritative immutable release points.

## v0.2.2 — 14 September 2026

### Centralized container awareness

- Nested services can now inherit repository-centralized Docker ownership when a Compose service points to one exact Dockerfile and that Dockerfile explicitly targets the nested service path.
- Added recognition for conventional `Dockerfile`, `Dockerfile.*`, and `*.Dockerfile` names.
- Prevented `fix <nested-service>` from proposing duplicate service-local Dockerfile, Compose, or `.dockerignore` files when centralized ownership is established.
- Container-security guidance now recognizes an inherited centralized Dockerfile.
- Ambiguous or unrelated centralized container definitions remain unassigned rather than guessed.
- Extended permanent monorepo regression coverage using the real MortgageOps centralized `infra/docker/api.Dockerfile` pattern.
- `readiness-v1` scoring weights remain unchanged.

## v0.2.1 — 14 September 2026

### Field-validation hardening

- Added repository-root awareness when inspecting a supported service nested inside a monorepo.
- Nested services now inherit repository-owned CI, environment conventions, dependency lockfiles, update automation, and security workflow signals while keeping runtime/Docker/health/Terraform evaluation service-local.
- `inspect` and `fix` now show both the target path and containing repository root for nested services.
- Prevented `fix <nested-service>` from generating unusable nested `.github/workflows` files.
- Repository-scoped security remediation is deferred when invoked from a nested service rather than written in the wrong location.
- Heterogeneous repository roots now report aggregate readiness explicitly and defer automatic `.stackpilot.toml` adoption instead of assigning one golden-path identity to multiple stacks.
- Added permanent monorepo-awareness regression coverage based on the real MortgageOps field-validation case.
- `readiness-v1` scoring weights remain unchanged.

## v0.2.0 — 13 September 2026

### Repository intelligence

- Added deterministic, read-only `stackpilot inspect` for existing repositories.
- Added the versioned `readiness-v1` score across Runtime, Delivery, Infrastructure, Security, and Operability.
- Added `--fail-below` so readiness can act as a CI quality gate.

### Safety-first remediation

- Added preview-first `stackpilot fix`; mutation requires `--apply`.
- Added deterministic remediation for environment hygiene, Docker, GitHub Actions CI, Terraform foundations, StackPilot metadata, and `/health` endpoints across all six backend golden paths.
- Added compare-before-write protection for application-source updates, symlink refusal, and ambiguity deferral.

### Security baseline

- Added ecosystem-aware Dependabot configuration.
- Added dependency, secret, and misconfiguration scanning with Trivy.
- Added CycloneDX SBOM generation and conditional container image scanning.
- Hardened generated GitHub Actions permissions.
- Added `stackpilot fix --security` for verified existing golden-path repositories.

### Deployment intelligence

- Added deterministic AWS ECS/Fargate recommendation for verified Dockerized AWS backend golden paths.
- Added `stackpilot fix --deployment aws-ecs-fargate`.
- Added additive Terraform for ECR, ECS/Fargate, ALB, IAM roles, CloudWatch logs, health checks, variables, and outputs while keeping VPC/subnet topology as explicit user inputs.

### Golden-path lifecycle

- Added independent `golden_path_version` metadata; V0.2 projects use golden path v2 while `.stackpilot.toml` schema remains version 1.
- Added preview-first `stackpilot upgrade` with explicit `--apply`.
- Added deterministic v1 → v2 migration for StackPilot-managed repositories.
- Legacy `True`/`False` metadata is normalized safely.
- Lifecycle upgrades preserve application source and refuse incompatible custom managed security files rather than overwriting them.

### Release quality

- `Cargo.lock` is committed and release/CI builds use `--locked`.
- Installed-release smoke coverage verifies Linux x86_64, macOS x86_64, macOS arm64, and Windows x86_64.
- Dedicated regression workflows cover security baseline, health remediation, Terraform remediation, AWS ECS/Fargate generation, and golden-path lifecycle upgrades.

## v0.1.2 — 10 September 2026

- Bundled the StackPilot recipe library with native release archives.
- Added installed-path recipe resolution so the CLI works outside the source repository.
- Added installed-release smoke coverage and completed real installed-runtime verification.

## v0.1.1

- Standardized semantic release tags, native archives, checksums, installer diagnostics, and repeatable GitHub release publishing.

## v0.1.0

- Established the Rust CLI, reusable recipe engine, template bootstrap, planning/diagnostics, and six backend golden paths.
