# gODtECH ecosystem

StackPilot, gODtECH FORGE, and gODtECH Steward are independent products with deliberately separated responsibilities.

```text
                         FORGE
               ORCHESTRATE / GOVERN / VERIFY
                            |
              +-------------+-------------+
              |                           |
              v                           v
         StackPilot                    Steward
          BUILD IT                 KEEP IT HEALTHY
              |                           |
              +-------------+-------------+
                            v
                       TARGET PROJECT
```

## Product boundaries

**StackPilot** owns project scaffolding, golden paths, recipe rendering, generated-project validation, and stack-aware readiness.

**Steward** owns deterministic repository housekeeping, health findings, stable scan/report contracts, and conservative remediation.

**FORGE** owns AI-assisted engineering orchestration, bounded context, risk classification, approvals, resumable workflows, evidence-aware delivery, and cross-tool governance.

## Integration model

StackPilot can optionally consume a versioned Steward scan report as observational input. It does not copy Steward rules, invoke Steward remediation, or fold Steward health into `readiness-v1`.

FORGE may orchestrate StackPilot and Steward through their public interfaces when a workflow needs both project scaffolding and repository-health evidence.

Neither StackPilot nor Steward is required for FORGE to operate, and no product should import another product's private implementation modules.

## Non-duplication rule

- Golden-path and stack-aware semantics belong in **StackPilot**.
- Generic repository housekeeping belongs in **Steward**.
- Cross-product workflow, governance, and evidence correlation belong in **FORGE**.
- Shared information crosses boundaries through stable, versioned public contracts.

## Public references

- [gODtECH FORGE](https://github.com/gODtECH-Ctl-Create/gODtECH-FORGE)
- [gODtECH Steward](https://github.com/gODtECH-Ctl-Create/gODtECH-Steward)
- [Steward npm package](https://www.npmjs.com/package/@godtech/steward)
