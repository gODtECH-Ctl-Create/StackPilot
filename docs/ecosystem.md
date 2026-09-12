# gODtECH ecosystem: StackPilot and Steward

StackPilot and gODtECH Steward are independent products with different canonical responsibilities.

```text
StackPilot                         Steward
BUILD IT                    KEEP IT HEALTHY
   |                              |
   +------------+-----------------+
                |
                v
           TARGET PROJECT
```

## Responsibilities

**StackPilot** owns project scaffolding, language/framework selection, golden paths, recipe rendering, generated-project validation, and stack-aware readiness.

**Steward** owns generic repository housekeeping, deterministic health findings, machine-readable scan/report contracts, and conservative remediation.

## Integration

StackPilot may consume a Steward `schemaVersion: 1` scan report as optional observational input:

```bash
steward scan . --json > steward-report.json
stackpilot inspect . --steward-report steward-report.json
```

StackPilot does not reimplement Steward rules, does not fold Steward findings into `readiness-v1`, and does not invoke Steward remediation.

Steward does not require StackPilot. StackPilot does not require Steward.

## Ecosystem relationship with FORGE

[gODtECH FORGE](https://github.com/gODtECH-Ctl-Create/gODtECH-FORGE) is the orchestration layer that may coordinate independent tools through public contracts. Forge can use StackPilot when a workflow needs a supported project foundation and can use Steward when repository health or maintenance is relevant.

The separation is intentional:

```text
FORGE      → orchestrate / govern / verify
StackPilot → scaffold / generate / validate a golden path
Steward    → inspect / explain / maintain repository health
```

## Public references

- [StackPilot](https://github.com/gODtECH-Ctl-Create/StackPilot)
- [StackPilot website](https://godtech-ctl-create.github.io/StackPilot/)
- [Steward](https://github.com/gODtECH-Ctl-Create/gODtECH-Steward)
- [Steward npm package](https://www.npmjs.com/package/@godtech/steward)
- [Steward v0.1.0](https://github.com/gODtECH-Ctl-Create/gODtECH-Steward/releases/tag/v0.1.0)
- [Steward integration contract](./steward-integration.md)
