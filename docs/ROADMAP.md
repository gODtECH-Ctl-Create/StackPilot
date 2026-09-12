# StackPilot Roadmap

StackPilot is gODtECH's opinionated project scaffolding and golden-path engine. Its primary responsibility is to create or adopt projects with known-good structural, runtime, delivery, and infrastructure defaults.

## Product boundary

StackPilot answers:

> **What is the safest supported starting shape for this project, and can I generate it consistently?**

StackPilot should own stack selection, golden paths, recipe rendering, generated-project validation, and stack-aware adoption/remediation.

It should not become a general-purpose repository maintenance engine.

## Near-term priorities

- [x] Golden-path scaffolding for supported ecosystems.
- [x] Deterministic repository inspection and readiness scoring.
- [x] Preview-first, safety-gated remediation for supported stack patterns.
- [ ] Stabilize V0.2 inspection and remediation as a separately versioned capability.
- [ ] Add explicit integration contracts for gODtECH Steward findings.
- [ ] Consume Steward's generic repository-health findings where they improve StackPilot inspection without duplicating Steward rules.
- [ ] Preserve StackPilot-specific readiness controls for stack validity, generated structure, runtime conventions, infrastructure, and operability.

## gODtECH ecosystem integration

StackPilot is independently usable and should remain so. It can be used as a GitHub template, command-line tool, or project generator without Forge or Steward.

When available, **gODtECH Steward** may provide lower-level repository-maintenance signals to StackPilot. The integration should be contract-based and optional:

```text
StackPilot
   |
   +--> Stack selection / recipe / generation
   |
   +--> StackPilot-specific readiness checks
   |
   +--> Steward adapter (optional)
           |
           +--> generic repository-health findings
           +--> safe remediation metadata
```

StackPilot must not copy Steward's generic housekeeping rule implementations. Conversely, Steward must not encode StackPilot's golden-path assumptions.

## Forge relationship

**gODtECH FORGE** is the orchestration framework. StackPilot is one of the deterministic tools Forge may invoke when a task requires scaffolding, stack selection, or supported project generation.

```text
FORGE
  |
  +--> StackPilot: create / adopt / validate project structure
  |
  +--> Steward: inspect / maintain / safely remediate repository health
  |
  +--> Other verification and intelligence tools
```

Forge integration should remain optional and should consume stable command or machine-readable contracts rather than internal Rust modules.

## Boundary rules

1. StackPilot owns **scaffolding and golden-path correctness**.
2. Steward owns **general repository/software housekeeping**.
3. Forge owns **orchestration, policy, workflow, context, and cross-tool decisions**.
4. Shared facts may be reused through stable machine-readable contracts.
5. A capability should have one canonical implementation. New StackPilot checks should not duplicate a generic Steward rule merely because StackPilot can also observe the same repository fact.

## Future direction

- Stable readiness model versions that can incorporate external maintenance signals without changing their semantics silently.
- A Steward adapter for existing-repository inspection and safe remediation recommendations.
- Forge-aware generation and adoption workflows that can call StackPilot and Steward as independent capabilities.
- Shared evidence contracts so Forge can correlate scaffolding, maintenance, and verification results.
