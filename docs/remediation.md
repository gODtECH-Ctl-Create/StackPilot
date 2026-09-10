# Repository remediation

> Development status: `stackpilot fix` is V0.2 work on `main` once merged and is not part of the current v0.1.2 binary release.

`stackpilot fix` is designed around a simple rule: **preview first, write only deterministic changes, and never guess when application code or deployment intent is unclear.**

## Preview by default

```bash
stackpilot fix
```

or for another repository:

```bash
stackpilot fix ../payments-api
```

The default mode is a dry-run. StackPilot inspects the repository and prints the safe changes it can make, but writes nothing.

Example shape:

```text
StackPilot fix
Repository: /workspace/payments-api
Mode: preview
Readiness before: 47/100

Safe automatic changes
+ create .env.example — add a safe environment-variable example without secret values
~ append .gitignore — protect local .env files while keeping .env.example commit-safe
+ create .stackpilot.toml — adopt the repository into StackPilot metadata without changing application code

Deferred stack-aware changes
! Docker — container generation needs a verified runtime entrypoint before StackPilot can write it safely
! CI/CD — CI generation needs a verified build/test command adapter for the detected stack
! Terraform — infrastructure generation needs an explicit cloud/deployment target rather than guessing
! Health check — health remediation can touch application routing and needs a stack-specific source adapter

Preview only: no files were changed. Re-run with --apply to write these safe changes.
```

## Apply the safe plan

```bash
stackpilot fix --apply
```

`--apply` currently writes only the first safe remediation layer:

- creates `.env.example` when no root safe example exists;
- creates or appends `.gitignore` so local `.env` files are ignored while `.env.example` remains commit-safe;
- creates `.stackpilot.toml` when the repository has not been adopted by StackPilot;
- never overwrites an existing `.env.example` or `.stackpilot.toml`;
- refuses to modify a symlinked `.gitignore`;
- re-inspects the repository after applying changes and reports the readiness score before and after.

The generated environment example contains only safe placeholder/default values. StackPilot does not copy values from a local `.env` file and does not attempt to extract secrets.

## Why Docker, CI, Terraform and health fixes are initially deferred

These changes can be dangerous when guessed:

- a Dockerfile needs the real runtime entrypoint and artifact layout;
- CI needs the repository's verified install/build/test commands;
- Terraform needs an explicit cloud and deployment target;
- health checks may require modifying application routing/source code.

The first `fix` slice therefore surfaces those controls as deferred rather than creating plausible-looking files that may not work. Stack-specific remediation adapters are the next part of the V0.2 remediation milestone.

## Safety contract

StackPilot remediation follows these rules:

1. Preview is the default. Mutation requires `--apply`.
2. Existing application source is not edited by the initial safe-fix layer.
3. Existing managed/configuration files are not overwritten blindly.
4. A planned create fails if the target appears before apply, preventing accidental replacement.
5. Symlinked files are not followed for mutation.
6. Ambiguous stack/deployment decisions are deferred instead of guessed.
7. Inspection runs again after apply so the user can see the measurable effect.

This contract is intentionally conservative. Later stack adapters should preserve the same preview/apply boundary and add their own explicit safety checks before they become eligible for automatic remediation.
