use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

const MAX_SCAN_DEPTH: usize = 6;
const MAX_TEXT_FILE_SIZE: u64 = 512 * 1024;
const IGNORED_DIRECTORIES: &[&str] = &[
    ".git",
    ".next",
    ".venv",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "vendor",
    "venv",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingStatus {
    Passed,
    Warning,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub category: &'static str,
    pub name: &'static str,
    pub status: FindingStatus,
    pub detail: String,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectionReport {
    pub root: PathBuf,
    pub languages: Vec<String>,
    pub frameworks: Vec<String>,
    pub findings: Vec<Finding>,
}

impl InspectionReport {
    #[cfg(test)]
    fn finding(&self, name: &str) -> Option<&Finding> {
        self.findings.iter().find(|finding| finding.name == name)
    }
}

pub fn inspect_repository(root: &Path) -> Result<InspectionReport> {
    if !root.exists() {
        bail!("repository path does not exist: {}", root.display());
    }
    if !root.is_dir() {
        bail!("repository path is not a directory: {}", root.display());
    }

    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve repository path {}", root.display()))?;
    let files = collect_files(&root)?;
    let languages = detect_languages(&root, &files);
    let frameworks = detect_frameworks(&files);
    let docker = detect_docker(&root, &files);
    let ci = detect_ci(&root, &files);
    let terraform = detect_terraform(&files);
    let health = detect_health_check(&files);
    let environment = detect_environment_hygiene(&root, &files);
    let stackpilot_metadata = files
        .iter()
        .any(|path| path.file_name().and_then(|name| name.to_str()) == Some(".stackpilot.toml"));

    let findings = vec![
        Finding {
            category: "Runtime",
            name: "Language",
            status: if languages.is_empty() {
                FindingStatus::Warning
            } else {
                FindingStatus::Passed
            },
            detail: if languages.is_empty() {
                "No supported language marker was detected".to_string()
            } else {
                languages.join(", ")
            },
            recommendation: languages.is_empty().then(|| {
                "Add or expose a supported language manifest so StackPilot can identify the runtime."
                    .to_string()
            }),
        },
        Finding {
            category: "Runtime",
            name: "Framework",
            status: if frameworks.is_empty() {
                FindingStatus::Warning
            } else {
                FindingStatus::Passed
            },
            detail: if frameworks.is_empty() {
                "No recognized framework marker was detected".to_string()
            } else {
                frameworks.join(", ")
            },
            recommendation: frameworks.is_empty().then(|| {
                "Expose framework dependencies in the project manifest so StackPilot can identify the application framework."
                    .to_string()
            }),
        },
        Finding {
            category: "Runtime",
            name: "Docker",
            status: if docker.detected {
                FindingStatus::Passed
            } else {
                FindingStatus::Missing
            },
            detail: docker.detail,
            recommendation: (!docker.detected)
                .then(|| "Add a production container definition such as a Dockerfile.".to_string()),
        },
        Finding {
            category: "Runtime",
            name: "Health check",
            status: if health {
                FindingStatus::Passed
            } else {
                FindingStatus::Missing
            },
            detail: if health {
                "Health/readiness endpoint convention detected".to_string()
            } else {
                "No health/readiness endpoint convention detected".to_string()
            },
            recommendation: (!health).then(|| {
                "Add a health endpoint such as /health and wire it into runtime/deployment checks."
                    .to_string()
            }),
        },
        Finding {
            category: "Delivery",
            name: "CI/CD",
            status: if ci.detected {
                FindingStatus::Passed
            } else {
                FindingStatus::Missing
            },
            detail: ci.detail,
            recommendation: (!ci.detected)
                .then(|| "Add a CI workflow that builds and tests the repository on every change.".to_string()),
        },
        Finding {
            category: "Infrastructure",
            name: "Terraform",
            status: if terraform > 0 {
                FindingStatus::Passed
            } else {
                FindingStatus::Missing
            },
            detail: if terraform > 0 {
                format!("{terraform} Terraform file(s) detected")
            } else {
                "No Terraform configuration detected".to_string()
            },
            recommendation: (terraform == 0).then(|| {
                "Add infrastructure-as-code when the service owns deployable infrastructure."
                    .to_string()
            }),
        },
        Finding {
            category: "Configuration",
            name: "Environment config",
            status: environment.status,
            detail: environment.detail,
            recommendation: environment.recommendation,
        },
        Finding {
            category: "Configuration",
            name: "StackPilot metadata",
            status: if stackpilot_metadata {
                FindingStatus::Passed
            } else {
                FindingStatus::Warning
            },
            detail: if stackpilot_metadata {
                ".stackpilot.toml detected".to_string()
            } else {
                ".stackpilot.toml not found".to_string()
            },
            recommendation: (!stackpilot_metadata).then(|| {
                "Add StackPilot project metadata so future remediation and upgrades can track repository intent."
                    .to_string()
            }),
        },
    ];

    Ok(InspectionReport {
        root,
        languages,
        frameworks,
        findings,
    })
}

#[derive(Debug)]
struct Detection {
    detected: bool,
    detail: String,
}

#[derive(Debug)]
struct EnvironmentDetection {
    status: FindingStatus,
    detail: String,
    recommendation: Option<String>,
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files_at(root, 0, &mut files)?;
    Ok(files)
}

fn collect_files_at(directory: &Path, depth: usize, files: &mut Vec<PathBuf>) -> Result<()> {
    if depth > MAX_SCAN_DEPTH {
        return Ok(());
    }

    let entries = fs::read_dir(directory)
        .with_context(|| format!("failed to read directory {}", directory.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| {
            format!("failed to read directory entry in {}", directory.display())
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", path.display()))?;

        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            let name = entry.file_name();
            if name
                .to_str()
                .is_some_and(|name| IGNORED_DIRECTORIES.contains(&name))
            {
                continue;
            }
            collect_files_at(&path, depth + 1, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }

    Ok(())
}

fn detect_languages(root: &Path, files: &[PathBuf]) -> Vec<String> {
    let mut languages = BTreeSet::new();
    let has_tsconfig = files.iter().any(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("tsconfig") && name.ends_with(".json"))
    });

    for path in files {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        match name {
            "Cargo.toml" => {
                languages.insert("Rust".to_string());
            }
            "go.mod" => {
                languages.insert("Go".to_string());
            }
            "package.json" => {
                languages.insert(
                    if has_tsconfig {
                        "TypeScript"
                    } else {
                        "JavaScript"
                    }
                    .to_string(),
                );
            }
            "pyproject.toml" | "requirements.txt" | "setup.py" => {
                languages.insert("Python".to_string());
            }
            "pom.xml" | "build.gradle" | "build.gradle.kts" => {
                languages.insert("Java".to_string());
            }
            _ if name.ends_with(".csproj") => {
                languages.insert("C#".to_string());
            }
            _ => {}
        }
    }

    if languages.is_empty() {
        let extensions: BTreeSet<String> = files
            .iter()
            .filter_map(|path| path.extension().and_then(|extension| extension.to_str()))
            .map(str::to_ascii_lowercase)
            .collect();

        for (markers, language) in [
            (&["rs"][..], "Rust"),
            (&["go"][..], "Go"),
            (&["ts", "tsx"][..], "TypeScript"),
            (&["js", "jsx", "mjs", "cjs"][..], "JavaScript"),
            (&["py"][..], "Python"),
            (&["java"][..], "Java"),
            (&["cs"][..], "C#"),
        ] {
            if markers
                .iter()
                .any(|extension| extensions.contains(*extension))
            {
                languages.insert(language.to_string());
            }
        }
    }

    if root.join("tsconfig.json").is_file() {
        languages.remove("JavaScript");
        languages.insert("TypeScript".to_string());
    }

    languages.into_iter().collect()
}

fn detect_frameworks(files: &[PathBuf]) -> Vec<String> {
    let mut frameworks = BTreeSet::new();

    for path in files {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let relevant = matches!(
            name,
            "Cargo.toml"
                | "go.mod"
                | "package.json"
                | "pyproject.toml"
                | "requirements.txt"
                | "pom.xml"
                | "build.gradle"
                | "build.gradle.kts"
        ) || name.ends_with(".csproj");

        if !relevant {
            continue;
        }

        let Some(content) = read_small_text(path) else {
            continue;
        };
        let content = content.to_ascii_lowercase();

        for (marker, framework) in [
            ("axum", "Axum"),
            ("actix-web", "Actix Web"),
            ("rocket", "Rocket"),
            ("github.com/go-chi/chi", "Chi"),
            ("github.com/gin-gonic/gin", "Gin"),
            ("github.com/labstack/echo", "Echo"),
            ("@nestjs/core", "NestJS"),
            ("\"express\"", "Express"),
            ("\"fastify\"", "Fastify"),
            ("\"next\"", "Next.js"),
            ("fastapi", "FastAPI"),
            ("django", "Django"),
            ("flask", "Flask"),
            ("spring-boot", "Spring Boot"),
            ("microsoft.net.sdk.web", "ASP.NET Core"),
            ("microsoft.aspnetcore", "ASP.NET Core"),
        ] {
            if content.contains(marker) {
                frameworks.insert(framework.to_string());
            }
        }
    }

    frameworks.into_iter().collect()
}

fn detect_docker(root: &Path, files: &[PathBuf]) -> Detection {
    let mut dockerfiles = Vec::new();
    let mut compose_files = Vec::new();

    for path in files {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();

        if lower == "dockerfile" || lower.starts_with("dockerfile.") {
            dockerfiles.push(relative_path(root, path));
        }
        if matches!(
            lower.as_str(),
            "compose.yml" | "compose.yaml" | "docker-compose.yml" | "docker-compose.yaml"
        ) {
            compose_files.push(relative_path(root, path));
        }
    }

    Detection {
        detected: !dockerfiles.is_empty() || !compose_files.is_empty(),
        detail: match (dockerfiles.first(), compose_files.first()) {
            (Some(dockerfile), Some(compose)) => {
                format!("Dockerfile and Compose detected ({dockerfile}, {compose})")
            }
            (Some(dockerfile), None) => format!("Dockerfile detected ({dockerfile})"),
            (None, Some(compose)) => format!("Compose detected ({compose})"),
            (None, None) => "No Dockerfile or Compose configuration detected".to_string(),
        },
    }
}

fn detect_ci(root: &Path, files: &[PathBuf]) -> Detection {
    let mut providers = BTreeSet::new();

    for path in files {
        let relative = relative_path(root, path);
        let normalized = relative.replace('\\', "/");
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if normalized.starts_with(".github/workflows/")
            && matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("yml" | "yaml")
            )
        {
            providers.insert("GitHub Actions");
        }

        match name {
            ".gitlab-ci.yml" => {
                providers.insert("GitLab CI");
            }
            "Jenkinsfile" => {
                providers.insert("Jenkins");
            }
            "azure-pipelines.yml" | "azure-pipelines.yaml" => {
                providers.insert("Azure Pipelines");
            }
            "bitbucket-pipelines.yml" | "bitbucket-pipelines.yaml" => {
                providers.insert("Bitbucket Pipelines");
            }
            _ => {}
        }
    }

    Detection {
        detected: !providers.is_empty(),
        detail: if providers.is_empty() {
            "No supported CI/CD configuration detected".to_string()
        } else {
            providers.into_iter().collect::<Vec<_>>().join(", ")
        },
    }
}

fn detect_terraform(files: &[PathBuf]) -> usize {
    files
        .iter()
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("tf"))
        .count()
}

fn detect_health_check(files: &[PathBuf]) -> bool {
    const HEALTH_MARKERS: &[&str] = &[
        "/health",
        "/healthz",
        "/ready",
        "/readiness",
        "/live",
        "/liveness",
    ];

    files.iter().any(|path| {
        is_source_or_config_file(path)
            && read_small_text(path).is_some_and(|content| {
                let content = content.to_ascii_lowercase();
                HEALTH_MARKERS.iter().any(|marker| content.contains(marker))
            })
    })
}

fn detect_environment_hygiene(root: &Path, files: &[PathBuf]) -> EnvironmentDetection {
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

fn gitignore_protects_env(content: &str) -> bool {
    content.lines().any(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            return false;
        }

        matches!(line, ".env" | ".env*" | "*.env" | "**/.env") || line.ends_with("/.env")
    })
}

fn is_source_or_config_file(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "rs" | "go"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "mjs"
            | "cjs"
            | "py"
            | "java"
            | "cs"
            | "json"
            | "toml"
            | "yml"
            | "yaml"
            | "xml"
            | "gradle"
    )
}

fn read_small_text(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > MAX_TEXT_FILE_SIZE {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{FindingStatus, inspect_repository};

    #[test]
    fn detects_nestjs_production_foundations() {
        let repo = tempdir().expect("repository");
        fs::write(
            repo.path().join("package.json"),
            r#"{"dependencies":{"@nestjs/core":"latest"},"devDependencies":{"typescript":"latest"}}"#,
        )
        .expect("package.json");
        fs::write(repo.path().join("tsconfig.json"), "{}").expect("tsconfig");
        fs::write(repo.path().join("Dockerfile"), "FROM node:24").expect("dockerfile");
        fs::write(repo.path().join("compose.yaml"), "services: {}").expect("compose");
        fs::write(repo.path().join(".env.example"), "PORT=3000\n").expect("env example");
        fs::write(repo.path().join(".gitignore"), ".env\n").expect("gitignore");
        fs::write(repo.path().join(".env"), "LOCAL_ONLY=value\n").expect("local env");
        fs::write(repo.path().join(".stackpilot.toml"), "version = 1\n")
            .expect("stackpilot metadata");

        fs::create_dir_all(repo.path().join("src")).expect("src");
        fs::write(
            repo.path().join("src/main.ts"),
            "app.get('/health', () => ({ status: 'ok' }));",
        )
        .expect("source");
        fs::create_dir_all(repo.path().join(".github/workflows")).expect("workflows");
        fs::write(repo.path().join(".github/workflows/ci.yml"), "name: CI").expect("workflow");
        fs::create_dir_all(repo.path().join("infra/terraform")).expect("terraform dir");
        fs::write(repo.path().join("infra/terraform/main.tf"), "terraform {}").expect("terraform");

        let report = inspect_repository(repo.path()).expect("inspection");

        assert!(report.languages.contains(&"TypeScript".to_string()));
        assert!(report.frameworks.contains(&"NestJS".to_string()));
        for name in [
            "Docker",
            "Health check",
            "CI/CD",
            "Terraform",
            "Environment config",
            "StackPilot metadata",
        ] {
            assert_eq!(
                report.finding(name).expect("finding").status,
                FindingStatus::Passed,
                "expected {name} to pass"
            );
        }
    }

    #[test]
    fn reports_missing_production_foundations_without_failing() {
        let repo = tempdir().expect("repository");
        fs::write(
            repo.path().join("package.json"),
            r#"{"dependencies":{"express":"latest"}}"#,
        )
        .expect("package.json");

        let report = inspect_repository(repo.path()).expect("inspection");

        assert!(report.languages.contains(&"JavaScript".to_string()));
        assert!(report.frameworks.contains(&"Express".to_string()));
        for name in ["Docker", "Health check", "CI/CD", "Terraform"] {
            assert_eq!(
                report.finding(name).expect("finding").status,
                FindingStatus::Missing,
                "expected {name} to be missing"
            );
        }
        assert_eq!(
            report
                .finding("Environment config")
                .expect("environment finding")
                .status,
            FindingStatus::Missing
        );
    }

    #[test]
    fn ignores_dependency_and_build_directories() {
        let repo = tempdir().expect("repository");
        fs::create_dir_all(repo.path().join("node_modules/example")).expect("node_modules");
        fs::write(
            repo.path().join("node_modules/example/package.json"),
            r#"{"dependencies":{"@nestjs/core":"latest"}}"#,
        )
        .expect("nested package");
        fs::create_dir_all(repo.path().join("target/debug")).expect("target");
        fs::write(
            repo.path().join("target/debug/generated.rs"),
            "fn main() {}",
        )
        .expect("generated source");
        fs::write(repo.path().join("README.md"), "# Empty repository").expect("readme");

        let report = inspect_repository(repo.path()).expect("inspection");
        assert!(report.languages.is_empty());
        assert!(report.frameworks.is_empty());
    }

    #[test]
    fn warns_when_dotenv_is_not_ignored() {
        let repo = tempdir().expect("repository");
        fs::write(repo.path().join(".env"), "SECRET=example\n").expect("env");
        fs::write(repo.path().join(".env.example"), "SECRET=\n").expect("env example");

        let report = inspect_repository(repo.path()).expect("inspection");
        assert_eq!(
            report
                .finding("Environment config")
                .expect("environment finding")
                .status,
            FindingStatus::Warning
        );
    }
}
