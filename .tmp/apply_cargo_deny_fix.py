from pathlib import Path
import re

path = Path('src/security.rs')
text = path.read_text()

pattern = re.compile(
    r'    let dependency_scan = contains_any\(\n'
    r'        &workflow_lower,\n'
    r'        &\[\n'
    r'.*?'
    r'        \],\n'
    r'    \);',
    re.S,
)
replacement = '''    let cargo_deny_advisories = (workflow_lower.contains("cargo-deny-action")
        || workflow_lower.contains("cargo deny"))
        && workflow_lower.contains("advisories");
    let dependency_scan = cargo_deny_advisories
        || contains_any(
            &workflow_lower,
            &[
                "dependency-review-action",
                "trivy-action",
                "cargo audit",
                "npm audit",
                "pnpm audit",
                "pip-audit",
                "osv-scanner",
                "dependency-check",
            ],
        );'''
text, count = pattern.subn(replacement, text, count=1)
if count != 1:
    raise SystemExit(f'dependency scan block replacement count: {count}')

marker = '''    #[test]\n    fn warns_on_broad_workflow_permissions() {'''
tests = '''    #[test]
    fn detects_cargo_deny_advisories_as_dependency_scan() {
        let repo = tempdir().expect("repository");
        fs::create_dir_all(repo.path().join(".github/workflows")).expect("workflows");
        fs::write(
            repo.path().join(".github/workflows/deny.yml"),
            "jobs:\\n  audit:\\n    steps:\\n      - uses: EmbarkStudios/cargo-deny-action@v2\\n        with:\\n          command: check advisories\\n",
        )
        .expect("workflow");

        let findings = inspect(repo.path()).expect("security inspection");
        assert_eq!(
            findings
                .iter()
                .find(|finding| finding.name == "Dependency scanning")
                .expect("dependency scanning")
                .status,
            FindingStatus::Passed
        );
    }

    #[test]
    fn does_not_treat_cargo_deny_without_advisories_as_dependency_scan() {
        let repo = tempdir().expect("repository");
        fs::create_dir_all(repo.path().join(".github/workflows")).expect("workflows");
        fs::write(
            repo.path().join(".github/workflows/deny.yml"),
            "jobs:\\n  licenses:\\n    steps:\\n      - uses: EmbarkStudios/cargo-deny-action@v2\\n        with:\\n          command: check licenses\\n",
        )
        .expect("workflow");

        let findings = inspect(repo.path()).expect("security inspection");
        assert_eq!(
            findings
                .iter()
                .find(|finding| finding.name == "Dependency scanning")
                .expect("dependency scanning")
                .status,
            FindingStatus::Missing
        );
    }

'''
if marker not in text:
    raise SystemExit('test insertion marker not found')
text = text.replace(marker, tests + marker, 1)
path.write_text(text)
