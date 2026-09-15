# StackPilot v0.2.6 — Rust/Axum field-test wrap-up

Date: 2026-09-15

## Scope

This field test exercised StackPilot v0.2.6 against an existing Rust/Axum repository on Windows x86_64. The goal was to validate the adoption path end to end rather than only verify generated-project scaffolding.

## Result

The benchmark completed successfully at **100/100 readiness-v1** after explicit security and AWS Terraform remediation.

Validated behavior:

- Windows installer and current-shell CLI usage
- Rust and Axum detection
- Docker and Compose detection
- GitHub Actions detection
- environment hygiene detection
- health/readiness endpoint detection
- safe `fix` preview behavior
- `.stackpilot.toml` adoption
- generic `fix --apply` idempotency
- explicit `--security` remediation
- ecosystem-aware Cargo + GitHub Actions Dependabot configuration
- dependency, secret, SBOM and container scanning detection
- GitHub Actions explicit-permissions detection
- security-remediation idempotency
- explicit `--cloud AWS` Terraform generation
- Terraform-remediation idempotency
- `terraform fmt -check`
- `terraform init`
- `terraform validate`

## Bugs confirmed by the field test

### 1. Adopted metadata could become stale after cloud remediation

Observed sequence:

1. adopt an existing repository, creating `.stackpilot.toml` with `cloud = "None"` and `terraform = false`;
2. later run `stackpilot fix . --cloud AWS --apply`;
3. Terraform is generated and readiness detects it, but the previously created metadata remains stale.

Wrap-up fix:

- explicit cloud remediation now synchronizes an existing regular `.stackpilot.toml` after Terraform is successfully present;
- the cloud value is normalized to `AWS`, `Azure`, or `GCP`;
- `[features].terraform` is synchronized to `true`;
- preview mode never writes metadata and reports the synchronization separately.

### 2. Terraform variable descriptions could expose an unrelated application package name

The Rust adapter derives an application package identity from `Cargo.toml`. In an adopted repository this can differ from the repository identity and produced descriptions such as `AWS region for stellarsend-backend` during an unrelated field test.

Wrap-up fix:

- provider variable descriptions are now identity-neutral and describe the generated StackPilot Terraform foundation instead of embedding an application/package name.

## Deferred to a later work session

The following are intentionally documented but not included in this wrap-up scope:

- make `stackpilot doctor` context-aware so it can report missing optional tools such as Terraform when they are relevant to the repository;
- design `readiness-v2` so supply-chain controls can contribute appropriate weight rather than remaining only additional findings;
- distinguish provider-foundation Terraform from meaningful deployable infrastructure when assigning infrastructure readiness;
- evaluate safe preview-first remediation for existing GitHub Actions workflows that lack explicit permissions, without blindly changing third-party or write-capable workflows;
- run the next cross-stack adoption benchmark, with TypeScript/NestJS as the preferred next target.

## Release/benchmark interpretation

A `100/100` readiness-v1 score means every control represented by the current model was detected. It should not be interpreted as a claim that the application has complete production infrastructure or that every possible supply-chain control has been implemented.

This document closes the Rust/Axum v0.2.6 field-test session. Further enhancements above should be handled separately so the benchmark remains reproducible and the wrap-up change stays narrowly scoped.
