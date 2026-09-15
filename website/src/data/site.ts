import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

export const site = {
  currentVersion: '0.2.6',
  releaseDate: '15 September 2026',
  goldenPathCount: 6,
  nativeReleaseTargetCount: 4,
  proofPoints: [
    {
      value: '6',
      label: 'golden paths',
      detail: 'Rust, Go, TypeScript, Python, Java and C#',
    },
    {
      value: '4',
      label: 'native targets',
      detail: 'Windows, Linux, macOS Intel and macOS Apple Silicon',
    },
    {
      value: '100',
      label: 'readiness points',
      detail: 'Versioned readiness-v1 across runtime, delivery, infrastructure, security and operability',
    },
    {
      value: 'v2',
      label: 'golden-path lifecycle',
      detail: 'Preview-first upgrades preserve application source while managed foundations evolve',
    },
  ],
  releases: [
    {
      version: 'v0.1.0',
      title: 'Foundation',
      summary:
        'Established the StackPilot CLI, reusable recipe engine, planning and diagnostics, GitHub template bootstrap, and the six backend golden paths.',
    },
    {
      version: 'v0.1.1',
      title: 'Release distribution',
      summary:
        'Standardized semantic release tags, native platform archives, checksums, installer diagnostics, and repeatable GitHub release publishing.',
    },
    {
      version: 'v0.1.2',
      title: 'Install-ready runtime',
      summary:
        'Bundled the recipe library with installed binaries, added installed-path resolution and regression coverage, and completed real installed-runtime verification.',
    },
    {
      version: 'v0.2.0',
      title: 'Repository engineering',
      summary:
        'Added deterministic repository inspection, readiness-v1 scoring, safety-first remediation, managed security foundations, AWS ECS/Fargate deployment intelligence, and golden-path v2 lifecycle upgrades.',
    },
    {
      version: 'v0.2.1',
      title: 'Monorepo awareness',
      summary:
        'Field validation hardened nested-service inspection: repository-owned controls are inherited safely, invalid nested CI placement is prevented, and heterogeneous roots defer ambiguous StackPilot adoption.',
    },
    {
      version: 'v0.2.2',
      title: 'Centralized container awareness',
      summary:
        'Nested services now recognize deterministic repository-centralized Dockerfile and Compose ownership, preventing duplicate container remediation and improving service readiness accuracy.',
    },
    {
      version: 'v0.2.3',
      title: 'Shared gODtECH CLI identity',
      summary:
        'Adds the canonical gODtECH terminal identity to interactive StackPilot commands while preserving clean non-interactive, help, version, and automation output.',
    },
    {
      version: 'v0.2.4',
      title: 'Repository-scope remediation safety',
      summary:
        'External FastAPI field validation hardened nested remediation so repository-owned environment findings remain visible but are deferred to repository scope instead of creating misleading service-local env files.',
    },
    {
      version: 'v0.2.5',
      title: 'File-based configuration awareness',
      summary:
        'External Go/Chi field validation expanded configuration hygiene beyond dotenv: safe YAML, TOML and JSON example-plus-ignore conventions are now recognized without inventing .env remediation.',
    },
    {
      version: 'v0.2.6',
      title: 'Rust advisory-scan awareness',
      summary:
        'External Rust/Axum field validation now recognizes cargo-deny advisory CI as dependency vulnerability scanning while keeping non-advisory cargo-deny checks from satisfying that control.',
    },
  ],
} as const;

export const currentTag = `v${site.currentVersion}`;
export const latestReleaseUrl = `https://github.com/gODtECH-Ctl-Create/StackPilot/releases/tag/${currentTag}`;

// Keep the public website release metadata aligned with the CLI package version.
// This works whether Astro is run from the repository root or from /website.
const cargoCandidates = [
  resolve(process.cwd(), 'Cargo.toml'),
  resolve(process.cwd(), '..', 'Cargo.toml'),
];
const cargoPath = cargoCandidates.find(existsSync);

if (cargoPath) {
  const cargoToml = readFileSync(cargoPath, 'utf8');
  const cargoVersion = cargoToml.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1];

  if (cargoVersion && cargoVersion !== site.currentVersion) {
    throw new Error(
      `Website release metadata is ${site.currentVersion}, but Cargo.toml is ${cargoVersion}. Update website/src/data/site.ts before publishing.`,
    );
  }
}
