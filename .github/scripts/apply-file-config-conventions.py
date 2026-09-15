from pathlib import Path

path = Path("src/inspect.rs")
text = path.read_text()
start_marker = "fn detect_environment_hygiene(root: &Path, files: &[PathBuf]) -> EnvironmentDetection {\n"
end_marker = "\nfn gitignore_protects_env(content: &str) -> bool {\n"
start = text.index(start_marker)
end = text.index(end_marker, start)
replacement = r'''fn detect_environment_hygiene(root: &Path, files: &[PathBuf]) -> EnvironmentDetection {
    if let Some(file_config) = detect_file_config_hygiene(root, files) {
        return file_config;
    }

    let example_files: Vec<String> = files
        .iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    matches!(
                        name.to_ascii_lowercase().as_str(),
                        ".env.example" | ".env.sample" | ".env.template" | "env.example"
                    )
                })
        })
        .map(|path| relative_path(root, path))
        .collect();

    let dotenv_files: Vec<String> = files
        .iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some(".env"))
        .map(|path| relative_path(root, path))
        .collect();

    let env_ignored = files.iter().any(|path| {
        path.file_name().and_then(|name| name.to_str()) == Some(".gitignore")
            && read_small_text(path).is_some_and(|content| gitignore_protects_env(&content))
    });

    match (example_files.first(), env_ignored, dotenv_files.first()) {
        (Some(example), true, Some(_)) => EnvironmentDetection {
            status: FindingStatus::Passed,
            detail: format!("Safe example detected and local .env is ignored ({example})"),
            recommendation: None,
        },
        (Some(example), true, None) => EnvironmentDetection {
            status: FindingStatus::Passed,
            detail: format!("Safe example detected and .env is ignored ({example})"),
            recommendation: None,
        },
        (Some(example), false, Some(dotenv)) => EnvironmentDetection {
            status: FindingStatus::Warning,
            detail: format!(
                "Environment example exists, but {dotenv} is not protected by a detected .gitignore rule ({example})"
            ),
            recommendation: Some(
                "Add .env to .gitignore and verify no secrets have been committed.".to_string(),
            ),
        },
        (Some(example), false, None) => EnvironmentDetection {
            status: FindingStatus::Warning,
            detail: format!(
                "Environment example exists, but .env ignore protection was not detected ({example})"
            ),
            recommendation: Some(
                "Add .env to .gitignore while keeping the safe example file committed.".to_string(),
            ),
        },
        (None, true, Some(_)) | (None, true, None) => EnvironmentDetection {
            status: FindingStatus::Warning,
            detail: ".env is ignored, but no safe example environment file was found".to_string(),
            recommendation: Some(
                "Add a .env.example containing variable names and safe placeholder values.".to_string(),
            ),
        },
        (None, false, Some(dotenv)) => EnvironmentDetection {
            status: FindingStatus::Warning,
            detail: format!("{dotenv} is present without detected .gitignore protection"),
            recommendation: Some(
                "Ignore .env, add a safe .env.example, and verify no secrets have been committed."
                    .to_string(),
            ),
        },
        (None, false, None) => EnvironmentDetection {
            status: FindingStatus::Missing,
            detail: "No environment example or .env ignore convention detected".to_string(),
            recommendation: Some(
                "Add .env.example and ignore local .env files to document configuration without committing secrets."
                    .to_string(),
            ),
        },
    }
}

fn detect_file_config_hygiene(root: &Path, files: &[PathBuf]) -> Option<EnvironmentDetection> {
    let mut examples = files
        .iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_file_config_example_name)
        })
        .map(|path| relative_path(root, path))
        .collect::<Vec<_>>();
    examples.sort();

    let example = examples.first()?;
    let runtime_config_ignored = files.iter().any(|path| {
        path.file_name().and_then(|name| name.to_str()) == Some(".gitignore")
            && read_small_text(path)
                .is_some_and(|content| gitignore_protects_file_config(&content))
    });

    Some(if runtime_config_ignored {
        EnvironmentDetection {
            status: FindingStatus::Passed,
            detail: format!(
                "Safe file-based configuration example detected and local runtime config is ignored ({example})"
            ),
            recommendation: None,
        }
    } else {
        EnvironmentDetection {
            status: FindingStatus::Warning,
            detail: format!(
                "File-based configuration example exists, but local runtime config ignore protection was not detected ({example})"
            ),
            recommendation: Some(
                "Ignore the local runtime config path (for example config/ or config.yml) while keeping the example configuration committed."
                    .to_string(),
            ),
        }
    })
}

fn is_file_config_example_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let Some((stem, extension)) = lower.rsplit_once('.') else {
        return false;
    };
    if !matches!(extension, "yml" | "yaml" | "toml" | "json") {
        return false;
    }

    matches!(
        stem,
        "example-config"
            | "sample-config"
            | "template-config"
            | "config.example"
            | "config.sample"
            | "config.template"
            | "config-example"
            | "config-sample"
            | "config-template"
    )
}

fn gitignore_protects_file_config(content: &str) -> bool {
    content.lines().any(|line| {
        let line = line.trim().replace('\\', "/");
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            return false;
        }
        let normalized = line.trim_start_matches("./").trim_start_matches('/');
        matches!(
            normalized,
            "config"
                | "config/"
                | "config.yml"
                | "config.yaml"
                | "config.toml"
                | "config.json"
        )
    })
}
'''
text = text[:start] + replacement + text[end:]

test_anchor = '''    #[test]\n    fn warns_when_dotenv_is_not_ignored() {\n'''
new_test = r'''    #[test]
    fn recognizes_safe_file_based_configuration_convention() {
        let repo = tempdir().expect("repository");
        fs::write(
            repo.path().join("example-config.yml"),
            "server:\n  publicAddress: https://example.com\n",
        )
        .expect("example config");
        fs::write(repo.path().join(".gitignore"), "/config/\n")
            .expect("gitignore");

        let report = inspect_repository(repo.path()).expect("inspection");
        let environment = report
            .finding("Environment config")
            .expect("environment finding");
        assert_eq!(environment.status, FindingStatus::Passed);
        assert!(environment.detail.contains("file-based configuration example"));
        assert!(environment.detail.contains("example-config.yml"));
    }

    #[test]
    fn file_based_configuration_without_runtime_ignore_is_a_warning() {
        let repo = tempdir().expect("repository");
        fs::write(repo.path().join("config.example.toml"), "port = 8080\n")
            .expect("example config");

        let report = inspect_repository(repo.path()).expect("inspection");
        let environment = report
            .finding("Environment config")
            .expect("environment finding");
        assert_eq!(environment.status, FindingStatus::Warning);
        assert!(
            environment
                .recommendation
                .as_deref()
                .is_some_and(|recommendation| recommendation.contains("runtime config path"))
        );
    }

'''
if test_anchor not in text:
    raise SystemExit("test anchor not found")
text = text.replace(test_anchor, new_test + test_anchor, 1)
path.write_text(text)
