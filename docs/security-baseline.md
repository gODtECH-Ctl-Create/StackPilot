# StackPilot security baseline

StackPilot V0.2 treats repository security as a deterministic engineering control, not an AI-generated score.

## Inspection

`stackpilot inspect` reports security findings alongside the existing production-readiness findings. The first security contract checks for:

- dependency lockfiles
- Dependabot or Renovate update automation
- dependency vulnerability scanning in CI
- secret scanning in CI
- SBOM generation
- container image scanning
- explicit GitHub Actions token permissions

These controls are reported without changing the `readiness-v1` 0–100 weighting. That keeps existing CI thresholds stable while the security contract matures.

## New golden paths

When CI is enabled for a generated StackPilot backend project, the base recipe also creates:

- `.github/workflows/security.yml`
- `.github/dependabot.yml`

The security workflow uses read-only repository permissions and provides:

1. Trivy filesystem scanning for vulnerabilities, secrets, and configuration issues.
2. CycloneDX SBOM generation and artifact upload.
3. Conditional container image scanning when a root `Dockerfile` exists.

Dependabot is configured for GitHub Actions plus the selected language ecosystem: Cargo, Go modules, npm, pip, Maven, or NuGet.

## Existing repositories

Preview the security baseline without writing files:

```bash
stackpilot fix --security
```

Apply the create-only baseline:

```bash
stackpilot fix --security --apply
```

Automatic security remediation is limited to verified StackPilot golden-path layouts. Existing security workflows and Dependabot/Renovate files are preserved instead of overwritten. Ambiguous repositories remain inspection-only until a safe adapter exists.

## Safety boundaries

- Preview remains the default.
- Existing security files are not replaced automatically.
- Symlinked target paths are rejected.
- Generated GitHub Actions workflows declare explicit `contents: read` permissions.
- StackPilot never records or prints secret values while inspecting security controls.
- Vulnerability and secret scanning is delegated to maintained scanners in CI rather than reimplemented in the StackPilot binary.

The initial baseline intentionally favors portable controls that work across public and private repositories without requiring GitHub Advanced Security features.
