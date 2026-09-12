from pathlib import Path

path = Path('src/inspect.rs')
text = path.read_text()
text = text.replace(
    '        is_source_or_config_file(path)\n            && read_small_text(path).is_some_and(|content| {',
    '        is_application_source_file(path)\n            && read_small_text(path).is_some_and(|content| {',
    1,
)
marker = 'fn is_source_or_config_file(path: &Path) -> bool {\n'
helper = '''fn is_application_source_file(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "rs" | "go" | "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "py" | "java" | "cs"
    )
}

'''
if marker not in text:
    raise SystemExit('source/config helper marker not found')
text = text.replace(marker, helper + marker, 1)
idx = text.rfind('\n}')
if idx == -1:
    raise SystemExit('test module end not found')
test = '''

    #[test]
    fn health_probe_in_config_does_not_replace_application_endpoint() {
        let repo = tempdir().expect("repository");
        fs::write(
            repo.path().join("compose.yaml"),
            "services:\n  app:\n    healthcheck:\n      test: [CMD, curl, http://localhost:3000/health]\n",
        )
        .expect("compose");
        fs::write(repo.path().join("main.go"), "package main\nfunc main() {}\n")
            .expect("source");

        let report = inspect_repository(repo.path()).expect("inspection");
        assert_eq!(
            report.finding("Health check").expect("health finding").status,
            FindingStatus::Missing
        );
    }
'''
text = text[:idx] + test + text[idx:]
path.write_text(text)
