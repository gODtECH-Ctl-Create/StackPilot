import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

export const site = {
  currentVersion: '0.1.2',
  releaseDate: '10 September 2026',
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
      value: '6/6',
      label: 'CI validated',
      detail: 'Every supported backend golden path is generated and checked in CI',
    },
    {
      value: 'Windows',
      label: 'end-to-end verified',
      detail: 'Install → doctor → recipes → plan → real project generation',
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
        'Bundled the recipe library with installed binaries, added installed-path resolution and regression coverage, and completed a real Windows end-to-end installation and project-generation test.',
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
