# Repository remediation

`stackpilot fix` ships in StackPilot v0.2.0 and follows a strict rule: **preview first, write only deterministic changes, and defer anything StackPilot cannot prove is safe for the detected repository.** Mutation always requires `--apply`.

## Preview by default

```bash
stackpilot fix
```

or for another repository:

```bash
stackpilot fix ../payments-api
```

The default mode is a dry-run. StackPilot inspects the repository and prints the changes it can make, but writes nothing.

## Apply the plan

```bash
stackpilot fix --apply
```

V0.2 supports deterministic remediation across repository hygiene, verified golden-path runtime/delivery files, explicit-cloud Terraform, health endpoints, security foundations, and the first deployment foundation.

## Repository hygiene and adoption

StackPilot can:

- create `.env.example` when no root safe example exists;
- create or append `.gitignore` so local `.env` files are ignored while `.env.example` remains commit-safe;
- create schema-safe `.stackpilot.toml` adoption metadata;
- re-inspect after apply and report the readiness score before and after.

The generated environment example contains only safe placeholder/default values. StackPilot does not copy values from a local `.env` file and does not attempt to extract secrets.

## Verified Docker and GitHub Actions adapters

When Docker or CI/CD is missing, StackPilot checks whether the repository matches exactly one supported golden-path stack **and** the file/build conventions required by StackPilot's templates are present. Only then can `fix` offer Docker and GitHub Actions generation.

Supported adapter pairs are:

| Language | Framework | Additional verification before generation |
| --- | --- | --- |
| Rust | Axum | root Cargo package and `src/main.rs` |
| Go | Chi | `go.mod` and root `main.go` |
| TypeScript | NestJS | `package.json`, `tsconfig.json`, `src/main.ts`, and build/start conventions producing `dist/main.js` |
| Python | FastAPI | `pyproject.toml`, dev extras, and `app/main.py` exposing an app convention |
| Java | Spring Boot | `pom.xml` producing `target/app.jar` |
| C# | ASP.NET Core | root `app.csproj` and `Program.cs` |

For an eligible repository, StackPilot can create missing:

- `Dockerfile`;
- `compose.yaml`;
- `.dockerignore`;
- `.github/workflows/ci.yml`.

The generated files come from the same tested `base` recipe templates used by StackPilot's greenfield golden paths. `fix` does not maintain a second independent set of Docker or CI templates.

If the default `recipes` directory is unavailable, installed StackPilot resolves recipes beside the executable just like `new`, `plan`, `bootstrap`, and `upgrade`. An explicit location can also be supplied:

```bash
stackpilot fix --recipes-dir /path/to/recipes
```

## Explicit-cloud Terraform remediation

Terraform remains opt-in for existing repositories. StackPilot will not guess a cloud target. Pass one of:

```bash
stackpilot fix --cloud AWS
stackpilot fix --cloud Azure
stackpilot fix --cloud GCP
```

The command still previews by default. Add `--apply` only after reviewing the plan. When Terraform is missing and the selected cloud is valid, StackPilot can create `infra/terraform/main.tf`, `variables.tf`, and `README.md` from the same tested recipe templates used for greenfield projects. Existing or unrecognized Terraform files are never overwritten. Symlinked target paths are refused.

## Verified health-check remediation

When `stackpilot inspect` reports that a supported backend has no health endpoint, `stackpilot fix` can plan a real `GET /health` endpoint for verified Axum, Chi, NestJS, FastAPI, Spring Boot, and ASP.NET Core layouts. It does not satisfy readiness by adding a comment or configuration marker: the remediation changes application routing or, for the standard Spring Boot layout, creates the tested health controller.

Health source edits use compare-before-write protection. StackPilot stores the exact source text used to create the preview and, during `--apply`, refuses the update if that file changed in the meantime. Ambiguous router/application startup patterns, unsupported package layouts, symlinked paths, or unknown stacks remain deferred rather than guessed.

## Security baseline remediation

For a verified golden-path repository, preview:

```bash
stackpilot fix . --security
```

and apply explicitly:

```bash
stackpilot fix . --security --apply
```

StackPilot can add the same managed security foundation used by CI-enabled V0.2 greenfield projects: ecosystem-aware Dependabot plus a hardened workflow for dependency/secret/misconfiguration scanning, CycloneDX SBOM generation, conditional container scanning, and explicit read-only workflow permissions. Existing targets are not overwritten blindly.

See [`security-baseline.md`](./security-baseline.md) for the control contract.

## AWS ECS/Fargate deployment foundation

For a verified Dockerized AWS backend golden path, preview:

```bash
stackpilot fix . --deployment aws-ecs-fargate
```

and apply explicitly:

```bash
stackpilot fix . --deployment aws-ecs-fargate --apply
```

The adapter generates additive Terraform for ECR, ECS/Fargate, ALB, IAM roles, CloudWatch logs, `/health`, variables, and outputs. Existing VPC/subnet IDs remain explicit inputs. StackPilot refuses conflicting cloud intent, unknown Terraform provider intent, partial managed deployment foundations, and target-file collisions.

See [`deployment-intelligence.md`](./deployment-intelligence.md) for the boundary and generated resources.

## What remains deferred

A remediation remains deferred when StackPilot cannot prove the required write is safe. Typical reasons include:

- more than one language or framework is detected;
- the detected language/framework pair is not a current StackPilot golden path;
- required runtime/build conventions cannot be verified;
- a target file already exists but is not recognized;
- a target path contains a symlink;
- Terraform is missing and no explicit cloud target was supplied;
- application routing/startup patterns are ambiguous for health remediation;
- an existing infrastructure/security/deployment foundation may have been customized beyond StackPilot's verified contract.

A deferred result is intentional. Detection alone is not treated as sufficient proof that an operational file can be generated safely.

## Safety contract

StackPilot remediation follows these rules:

1. Preview is the default. Mutation requires `--apply`.
2. Existing operational/configuration targets are not overwritten blindly.
3. A planned create fails if the target appears before apply, preventing accidental replacement.
4. Application-source updates use compare-before-write protection and fail if the source changed after planning.
5. Mutation paths are repository-relative and may not escape the repository root.
6. StackPilot refuses to write through symlinked files or parent directories.
7. Docker/CI/health/security/deployment generation requires verified stack or infrastructure evidence appropriate to that adapter.
8. Ambiguous cloud, deployment, or source decisions are deferred instead of guessed.
9. Inspection runs again after apply so the user can see the measurable readiness effect.
10. `readiness-v1` remains stable; additional V0.2 security/deployment findings do not silently change its 0–100 contract.

The same safety model is used by lifecycle upgrades through `stackpilot upgrade`, but lifecycle migrations are even stricter: they require StackPilot-managed metadata and only change explicitly versioned managed foundations.
