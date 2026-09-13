# Golden-path lifecycle upgrades

StackPilot-generated repositories now carry two independent version concepts:

- top-level `version` is the `.stackpilot.toml` profile schema version;
- `[stackpilot].golden_path_version` identifies the managed engineering standard used by the project.

Keeping these separate lets StackPilot evolve golden-path foundations without pretending every project-standard change is a metadata-schema breaking change.

## Current lifecycle

The current StackPilot golden path is **v2**.

New projects record:

```toml
version = 1

[stackpilot]
recipe = "base"
golden_path_version = 2
```

Projects generated before lifecycle versioning do not contain `golden_path_version`. StackPilot treats those managed projects as **golden path v1**.

## Preview an upgrade

```bash
stackpilot upgrade .
```

Preview is the default. StackPilot reports the current and target golden-path versions and the managed files it would create or update.

Apply explicitly:

```bash
stackpilot upgrade . --apply
```

When StackPilot is installed with bundled recipes, the normal installed recipe directory is resolved automatically. Source checkouts or custom recipe locations can use:

```bash
stackpilot upgrade . --recipes-dir ./recipes
```

## v1 -> v2 migration

The first lifecycle migration moves a legacy StackPilot-managed golden path to the V0.2 secure managed standard.

It can:

- normalize the old uppercase `True` / `False` metadata values emitted by early StackPilot releases;
- add `golden_path_version = 2` to the managed profile;
- when CI is enabled, add the StackPilot Security Baseline workflow and ecosystem-aware Dependabot policy when those targets are absent;
- recognize an already compatible custom security foundation instead of replacing it.

The migration does **not** rewrite application source code.

## Safety model

`stackpilot upgrade` is deliberately stricter than a general repair command:

- `.stackpilot.toml` is required; arbitrary repositories cannot be adopted implicitly through `upgrade`;
- only known StackPilot recipe/schema versions and supported golden-path language/framework pairs are eligible;
- preview is non-mutating and `--apply` is required;
- symlinked managed targets are refused;
- application source files are never part of the v1 -> v2 plan;
- existing custom security/dependency files are never overwritten;
- incompatible custom security automation blocks the version bump until the repository is aligned deliberately;
- apply re-checks `.stackpilot.toml` and refuses if it changed after planning;
- a project claiming a newer golden path than the installed StackPilot understands fails clearly instead of being downgraded.

This creates a forward lifecycle contract without turning StackPilot into a generic repository maintenance engine. Generic housekeeping remains the responsibility of gODtECH Steward; cross-product policy/orchestration remains the responsibility of gODtECH FORGE.

## Future migrations

Future golden-path releases can add explicit migration steps from v2 onward. Each migration should remain deterministic, versioned, previewable and limited to StackPilot-owned or mechanically verified foundation changes. Application/business logic should never be rewritten merely to make a version number advance.
