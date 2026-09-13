use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::inspect::{Finding, FindingStatus};

const LOCKFILES: &[&str] = &[
    "Cargo.lock",
    "go.sum",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lock",
    "bun.lockb",
    "poetry.lock",
    "uv.lock",
    "Pipfile.lock",
    "packages.lock.json",
];

pub fn inspect(root: &Path) -> Result<Vec<Finding>> {
    let workflow_text = read_workflows(root)?;
    let workflow_lower = workflow_text.to_ascii_lowercase();

    let lockfiles = LOCKFILES
        .iter()
        .filter(|name| root.join(name).is_file())
        .copied()
        .collect::<Vec<_>>();

    let dependabot = root.join(".github/dependabot.yml").is_file()
        || root.join(".github/dependabot.yaml").is_file()
        || root.join("renovate.json").is_file()
        || root.join("renovate.json5").is_file()
        || root.join(".renovaterc").is_file()
        || root.join(".renovaterc.json").is_file();

    let dependency_scan = contains_any(
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
    );
    let secret_scan = contains_any(
        &workflow_lower,
        &["gitleaks", "trufflehog", "detect-secrets"],
    ) || (workflow_lower.contains("trivy-action")
        && workflow_lower.contains("secret"));
    let sbom = contains_any(
        &workflow_lower,
        &["sbom-action", "cyclonedx", "syft", "spdx-json", "spdx"],
    );
    let container_scan = contains_any(&workflow_lower, &["anchore/scan-action", "grype"])
        || (workflow_lower.contains("trivy-action")
            && (workflow_lower.contains("scan-type: image")
                || workflow_lower.contains("scan-type: 'image'")
                || workflow_lower.contains("scan-type: \"image\"")
                || workflow_lower.contains("image-ref:")));

    let workflow_permissions = inspect_workflow_permissions(root)?;

    Ok(vec![
        finding(
            "Dependency lockfile",
            !lockfiles.is_empty(),
            if lockfiles.is_empty() {
                "No supported dependency lockfile detected".to_string()
            } else {
                format!("Lockfile detected ({})", lockfiles.join(", "))
            },
            "Commit the ecosystem lockfile so CI and releases resolve repeatable dependency versions.",
        ),
        finding(
            "Dependency update automation",
            dependabot,
            if dependabot {
                "Dependabot or Renovate configuration detected".to_string()
            } else {
                "No Dependabot or Renovate configuration detected".to_string()
            },
            "Add Dependabot or Renovate for automated dependency update pull requests.",
        ),
        finding(
            "Dependency scanning",
            dependency_scan,
            if dependency_scan {
                "Dependency vulnerability scanning detected in CI".to_string()
            } else {
                "No dependency vulnerability scanning detected in CI".to_string()
            },
            "Add a CI vulnerability scan such as Trivy filesystem scanning, OSV Scanner, or GitHub dependency review.",
        ),
        finding(
            "Secret scanning",
            secret_scan,
            if secret_scan {
                "Secret scanning detected in CI".to_string()
            } else {
                "No CI secret scanning workflow detected".to_string()
            },
            "Add deterministic secret scanning such as Trivy, Gitleaks, or TruffleHog without storing secret values.",
        ),
        finding(
            "SBOM generation",
            sbom,
            if sbom {
                "SBOM generation detected in CI".to_string()
            } else {
                "No SBOM generation detected in CI".to_string()
            },
            "Generate a CycloneDX or SPDX software bill of materials during CI.",
        ),
        finding(
            "Container scanning",
            container_scan,
            if container_scan {
                "Container image scanning detected in CI".to_string()
            } else if root.join("Dockerfile").is_file() {
                "Dockerfile detected without a container image scan".to_string()
            } else {
                "No container image scan detected; repository does not expose a root Dockerfile"
                    .to_string()
            },
            "When a Dockerfile is present, build and scan the image for high/critical vulnerabilities in CI.",
        ),
        Finding {
            category: "Security",
            name: "GitHub Actions permissions",
            status: workflow_permissions.status,
            detail: workflow_permissions.detail,
            recommendation: workflow_permissions.recommendation,
        },
    ])
}

fn finding(
    name: &'static str,
    passed: bool,
    detail: String,
    recommendation: &'static str,
) -> Finding {
    Finding {
        category: "Security",
        name,
        status: if passed {
            FindingStatus::Passed
        } else {
            FindingStatus::Missing
        },
        detail,
        recommendation: (!passed).then(|| recommendation.to_string()),
    }
}

#[derive(Debug)]
struct PermissionDetection {
    status: FindingStatus,
    detail: String,
    recommendation: Option<String>,
}

fn inspect_workflow_permissions(root: &Path) -> Result<PermissionDetection> {
    let workflow_dir = root.join(".github/workflows");
    if !workflow_dir.is_dir() {
        return Ok(PermissionDetection {
            status: FindingStatus::Missing,
            detail: "No GitHub Actions workflows detected for permission evaluation".to_string(),
            recommendation: Some(
                "Use explicit least-privilege permissions when adding GitHub Actions workflows."
                    .to_string(),
            ),
        });
    }

    let mut total = 0usize;
    let mut explicit = 0usize;
    let mut broad = Vec::new();

    for entry in fs::read_dir(&workflow_dir)
        .with_context(|| format!("failed to read {}", workflow_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file()
            || !matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("yml" | "yaml")
            )
        {
            continue;
        }

        total += 1;
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let lower = content.to_ascii_lowercase();
        if lower
            .lines()
            .any(|line| line.trim_start().starts_with("permissions:"))
        {
            explicit += 1;
        }
        if lower.contains("permissions: write-all")
            || lower.contains("contents: write")
            || lower.contains("actions: write")
        {
            broad.push(
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("workflow")
                    .to_string(),
            );
        }
    }

    if total == 0 {
        return Ok(PermissionDetection {
            status: FindingStatus::Missing,
            detail: "No GitHub Actions workflow files detected".to_string(),
            recommendation: Some(
                "Use explicit least-privilege permissions when adding GitHub Actions workflows."
                    .to_string(),
            ),
        });
    }

    if !broad.is_empty() {
        return Ok(PermissionDetection {
            status: FindingStatus::Warning,
            detail: format!(
                "Potential write-capable GitHub Actions permissions detected ({})",
                broad.join(", ")
            ),
            recommendation: Some(
                "Review write permissions and reduce GITHUB_TOKEN access to the minimum each job requires."
                    .to_string(),
            ),
        });
    }

    if explicit == total {
        Ok(PermissionDetection {
            status: FindingStatus::Passed,
            detail: format!("Explicit permissions detected in all {total} workflow(s)"),
            recommendation: None,
        })
    } else {
        Ok(PermissionDetection {
            status: FindingStatus::Warning,
            detail: format!(
                "Explicit permissions detected in {explicit}/{total} GitHub Actions workflow(s)"
            ),
            recommendation: Some(
                "Declare an explicit permissions block in every workflow and default to contents: read."
                    .to_string(),
            ),
        })
    }
}

fn read_workflows(root: &Path) -> Result<String> {
    let workflow_dir = root.join(".github/workflows");
    if !workflow_dir.is_dir() {
        return Ok(String::new());
    }

    let mut combined = String::new();
    for entry in fs::read_dir(&workflow_dir)
        .with_context(|| format!("failed to read {}", workflow_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("yml" | "yaml")
            )
        {
            combined.push_str(&fs::read_to_string(&path).unwrap_or_default());
            combined.push('\n');
        }
    }
    Ok(combined)
}

fn contains_any(content: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| content.contains(marker))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::inspect;
    use crate::inspect::FindingStatus;

    #[test]
    fn detects_complete_security_baseline() {
        let repo = tempdir().expect("repository");
        fs::create_dir_all(repo.path().join(".github/workflows")).expect("workflows");
        fs::write(repo.path().join("Cargo.lock"), "# lock\n").expect("lockfile");
        fs::write(
            repo.path().join(".github/dependabot.yml"),
            "version: 2\nupdates: []\n",
        )
        .expect("dependabot");
        fs::write(
            repo.path().join(".github/workflows/security.yml"),
            "permissions:\n  contents: read\n# aquasecurity/trivy-action\n# scanners: vuln,secret\n# format: cyclonedx\n# scan-type: image\n",
        )
        .expect("security workflow");

        let findings = inspect(repo.path()).expect("security inspection");
        for name in [
            "Dependency lockfile",
            "Dependency update automation",
            "Dependency scanning",
            "Secret scanning",
            "SBOM generation",
            "Container scanning",
            "GitHub Actions permissions",
        ] {
            assert_eq!(
                findings
                    .iter()
                    .find(|finding| finding.name == name)
                    .expect(name)
                    .status,
                FindingStatus::Passed,
                "expected {name} to pass"
            );
        }
    }

    #[test]
    fn warns_on_broad_workflow_permissions() {
        let repo = tempdir().expect("repository");
        fs::create_dir_all(repo.path().join(".github/workflows")).expect("workflows");
        fs::write(
            repo.path().join(".github/workflows/ci.yml"),
            "permissions: write-all\n",
        )
        .expect("workflow");

        let findings = inspect(repo.path()).expect("security inspection");
        assert_eq!(
            findings
                .iter()
                .find(|finding| finding.name == "GitHub Actions permissions")
                .expect("permissions")
                .status,
            FindingStatus::Warning
        );
    }
}
