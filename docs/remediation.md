# Repository remediation

> Development status: `stackpilot fix` is V0.2 work on `main` once merged and is not part of the current v0.1.2 binary release.

`stackpilot fix` follows a strict rule: **preview first, write only deterministic changes, and defer anything StackPilot cannot prove is safe for the detected repository.**

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

`--apply` can currently write two classes of deterministic remediation.

### Repository hygiene and adoption

StackPilot can:

- create `.env.example` when no root safe example exists;
- create or append `.gitignore` so local `.env` files are ignored while `.env.example` remains commit-safe;
- create schema-safe `.stackpilot.toml` adoption metadata;
- re-inspect after apply and report the readiness score before and after.

The generated environment example contains only safe placeholder/default values. StackPilot does not copy values from a local `.env` file and does not attempt to extract secrets.

### Verified Docker and GitHub Actions adapters

When Docker or CI/CD is missing, StackPilot now checks whether the repository matches exactly one supported golden-path stack **and** the file/build conventions required by StackPilot's templates are present. Only then can `fix` offer Docker and GitHub Actions generation.

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

If the default `recipes` directory is unavailable, installed StackPilot resolves recipes beside the executable just like `new`, `plan`, and `bootstrap`. An explicit location can also be supplied:

```bash
stackpilot fix --recipes-dir /path/to/recipes
```

## What remains deferred

StackPilot still defers a remediation when any of these conditions is true:

- more than one language or framework is detected;
- the detected language/framework pair is not a current StackPilot golden path;
- required runtime/build conventions cannot be verified;
- a target file already exists but is not recognized, because StackPilot will not replace it blindly;
- the GitHub Actions target path contains a symlink;
- Terraform is missing, because infrastructure generation still requires an explicit cloud/deployment target;
- a health check is missing, because adding one can require application-source routing changes.

A deferred result is intentional. Detection alone is not treated as sufficient proof that an operational file can be generated safely.

## Example

```text
StackPilot fix
Repository: /workspace/payments-api
Mode: preview
Readiness before: 40/100

Safe automatic changes
+ create .env.example — add a safe environment-variable example without secret values
+ create .gitignore — protect local .env files while keeping .env.example commit-safe
+ create Dockerfile — generate the verified golden-path production container definition
+ create compose.yaml — generate the verified local container orchestration definition
+ create .dockerignore — generate container build-context exclusions
+ create .github/workflows/ci.yml — generate the verified golden-path GitHub Actions build/test workflow
+ create .stackpilot.toml — adopt the repository into StackPilot metadata without changing application code

Deferred stack-aware changes
! Terraform — infrastructure generation needs an explicit cloud/deployment target rather than guessing
! Health check — health remediation can touch application routing and needs a stack-specific source adapter

Preview only: no files were changed. Re-run with --apply to write these safe changes.
```

## Safety contract

StackPilot remediation follows these rules:

1. Preview is the default. Mutation requires `--apply`.
2. Existing application source is not edited by the current remediation layers.
3. Existing operational/configuration targets are not overwritten blindly.
4. A planned create fails if the target appears before apply, preventing accidental replacement.
5. Mutation paths are repository-relative and may not escape the repository root.
6. StackPilot refuses to write through symlinked parent directories.
7. Docker/CI generation requires an exact supported stack plus verified file/build conventions.
8. Ambiguous deployment or application-source decisions are deferred instead of guessed.
9. Inspection runs again after apply so the user can see the measurable readiness effect.

Future Terraform, health-check, and CI-repair adapters must preserve this preview/apply boundary and add their own explicit safety checks before they become eligible for automatic remediation.
