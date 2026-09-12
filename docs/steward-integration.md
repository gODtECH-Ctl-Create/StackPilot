# gODtECH Steward integration

StackPilot can optionally consume a machine-readable scan produced by [gODtECH Steward](https://github.com/gODtECH-Ctl-Create/gODtECH-Steward).

The current public Steward release is [`@godtech/steward@0.1.0`](https://www.npmjs.com/package/@godtech/steward), and its scan result is versioned as `schemaVersion: 1`.

See [`docs/ecosystem.md`](./ecosystem.md) for the broader StackPilot, Steward, and FORGE boundary.

## Why the integration exists

The two tools own different responsibilities:

- **StackPilot** owns scaffolding, golden paths, stack-aware validation, and the `readiness-v1` model.
- **Steward** owns generic repository housekeeping, health findings, and conservative remediation.

StackPilot does not reimplement Steward rules and does not include Steward findings in the `readiness-v1` score.

## Usage

Generate a Steward report from the repository:

```bash
npx @godtech/steward@0.1.0 scan . --json > steward-report.json
```

Then inspect the repository with StackPilot and display the Steward result alongside the StackPilot readiness report:

```bash
stackpilot inspect . --steward-report steward-report.json
```

The option is also available when inspecting another repository:

```bash
stackpilot inspect ../payments-api --steward-report ../payments-api/steward-report.json
```

## Contract

The report must identify itself as:

```json
{
  "schemaVersion": 1,
  "tool": "gODtECH Steward",
  "version": 1
}
```

StackPilot validates those compatibility fields before showing the report. Steward's public scan contract is versioned independently, so incompatible future versions fail clearly instead of being silently interpreted.

The displayed Steward section includes only observational data needed by StackPilot:

- health score;
- finding count;
- fixable finding count;
- severity summary;
- active rule packs.

Finding details, remediation text, source content, and detected secret values are not consumed by this adapter.

## Boundaries

This integration is:

- **Optional**. StackPilot works normally without a Steward report.
- **Offline-friendly**. The adapter reads a local JSON file and does not call a Steward service.
- **Observational**. StackPilot does not invoke Steward remediation.
- **Deterministic**. The same valid Steward report produces the same displayed summary.
- **Independent**. StackPilot does not depend on Steward at build or runtime beyond the JSON contract.

Do not use the Steward section as a replacement for the StackPilot readiness score. They answer different questions.
