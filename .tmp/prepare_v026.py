from pathlib import Path

changelog = Path('CHANGELOG.md')
text = changelog.read_text()
entry = '''## v0.2.6 — 15 September 2026

### Rust dependency-scan awareness

- External Rust/Axum field validation against `StellarSend/backend` exposed a false negative where a real `cargo-deny` RustSec advisory workflow was not recognized as dependency vulnerability scanning.
- StackPilot now recognizes `cargo-deny` CI when the workflow explicitly enables the `advisories` check.
- Licenses/bans/sources-only `cargo-deny` jobs do not satisfy dependency vulnerability scanning.
- Existing dependency-scanner detection remains unchanged.
- Added positive and negative regression coverage for advisory-capable versus non-advisory `cargo-deny` workflows.
- `readiness-v1` scoring weights remain unchanged.

'''
if '## v0.2.6 — 15 September 2026' not in text:
    marker = '## v0.2.5 — 15 September 2026\n'
    if marker not in text:
        raise SystemExit('v0.2.5 changelog marker not found')
    text = text.replace(marker, entry + marker, 1)
    changelog.write_text(text)

site = Path('website/src/data/site.ts')
text = site.read_text()
text = text.replace("currentVersion: '0.2.5'", "currentVersion: '0.2.6'", 1)
release_entry = '''    {
      version: 'v0.2.6',
      title: 'Rust advisory-scan awareness',
      summary:
        'External Rust/Axum field validation now recognizes cargo-deny advisory CI as dependency vulnerability scanning while keeping non-advisory cargo-deny checks from satisfying that control.',
    },
'''
if "version: 'v0.2.6'" not in text:
    marker = '''    {
      version: 'v0.2.5',
      title: 'File-based configuration awareness',
      summary:
        'External Go/Chi field validation expanded configuration hygiene beyond dotenv: safe YAML, TOML and JSON example-plus-ignore conventions are now recognized without inventing .env remediation.',
    },
'''
    if marker not in text:
        raise SystemExit('v0.2.5 site release marker not found')
    text = text.replace(marker, marker + release_entry, 1)
site.write_text(text)
