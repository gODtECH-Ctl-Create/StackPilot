# StackPilot roadmap

## Current product

StackPilot is a Rust-powered project scaffolding and golden-path engine. Its primary responsibility is creating and evolving production-minded project structure without hiding stack-specific decisions inside generic tooling.

## Integration architecture

StackPilot and gODtECH Steward remain independent products with a contract boundary:

```text
StackPilot
  ├── scaffolding
  ├── golden paths
  ├── stack-aware validation
  └── readiness-v1
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

## Near-term

- [x] Complete the optional Steward report adapter and end-to-end verification.
- [x] Document the integration contract and privacy boundary.
- [x] Keep locked, reproducible Rust builds.
- [ ] Continue strengthening golden-path generation and stack-aware remediation.

## Later

- Optional richer cross-tool health views when the contracts are mature.
- Forge orchestration support where StackPilot is asked to scaffold and Steward is asked to assess repository health.
- Expanded golden paths only where recipe correctness and verification coverage justify them.

## Non-goals

- Turning StackPilot into a generic repository housekeeping engine.
- Reimplementing Steward rules.
- Making Steward a StackPilot runtime dependency.
- Folding Steward health directly into `readiness-v1` without a separately versioned model and explicit architectural decision.

## Completed integration

StackPilot now supports:

```bash
stackpilot inspect . --steward-report steward-report.json
```

The adapter validates the Steward identity/version contract, displays safe health summaries, leaves `readiness-v1` unchanged, and never executes Steward remediation. The integration is local, offline-friendly, and independently deployable.
