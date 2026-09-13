# StackPilot roadmap

## V0.2 product

StackPilot is a Rust-powered golden-path repository engineering tool. It can create opinionated backend projects, inspect existing repositories, calculate a deterministic readiness score, preview/apply verified remediation, generate a first production deployment foundation, and evolve StackPilot-managed foundations through versioned lifecycle upgrades.

The V0.2 release contract includes:

- six backend golden paths: Rust/Axum, Go/Chi, TypeScript/NestJS, Python/FastAPI, Java/Spring Boot, and C#/ASP.NET Core;
- `stackpilot inspect` with the versioned `readiness-v1` 0–100 model and CI threshold support;
- preview-first `stackpilot fix` for environment hygiene, Docker, CI, explicit-cloud Terraform, health checks, security foundations, and StackPilot metadata;
- managed security controls covering dependency updates/scanning, secret scanning, SBOM generation, conditional container scanning, and least-privilege GitHub Actions permissions;
- deterministic AWS ECS/Fargate recommendation and additive Terraform deployment generation;
- golden-path v2 lifecycle metadata plus preview-first `stackpilot upgrade` for source-preserving managed migrations;
- optional local Steward report observation without changing `readiness-v1`;
- locked cross-platform release builds and installed-release verification on four native targets.

## Ecosystem boundary

StackPilot and gODtECH Steward remain independent products with a contract boundary:

```text
StackPilot
  ├── scaffolding + golden paths
  ├── stack-aware inspection/readiness
  ├── deterministic remediation
  ├── deployment foundations
  └── managed golden-path lifecycle
          │
          │ optional observation
          ▼
   gODtECH Steward
  ├── repository housekeeping
  ├── health findings
  ├── deterministic evidence
  └── conservative remediation
```

StackPilot may consume Steward's public `schemaVersion: 1` scan result. It must not copy Steward's generic rules, change the meaning of `readiness-v1`, or invoke Steward remediation as its own behavior.

FORGE remains the higher-level orchestration/governance layer when a workflow needs coordinated use of StackPilot and Steward.

## Post-V0.2 priorities

- Improve terminal presentation, error messages, progress feedback, and `doctor` guidance.
- Add richer public demos, generated examples, changelog/roadmap surfaces, and release documentation.
- Evaluate AWS App Runner where it provides a materially simpler golden path than ECS/Fargate.
- Add Azure Container Apps and Google Cloud Run only with the same deterministic generation and CI validation standard as ECS/Fargate.
- Define production criteria before considering Kubernetes/EKS generation; do not add Kubernetes for breadth alone.
- Extend lifecycle migrations only through explicit versioned steps that preserve application/business logic.
- Consider a future readiness model version only when new scored controls justify changing the published 0–100 contract.

## Non-goals

- Turning StackPilot into a hosted deployment control plane.
- Turning StackPilot into a generic repository housekeeping engine.
- Reimplementing Steward rules or making Steward a StackPilot runtime dependency.
- Adding languages, frontends, cloud targets, or Kubernetes merely to increase feature count.
- Rewriting application/business logic just to advance a golden-path version.
- Silently changing `readiness-v1` weights as new informational checks are added.

## Completed Steward integration

StackPilot supports:

```bash
stackpilot inspect . --steward-report steward-report.json
```

The adapter validates the Steward identity/version contract, displays safe health summaries, leaves `readiness-v1` unchanged, and never executes Steward remediation. The integration is local, offline-friendly, and independently deployable.
