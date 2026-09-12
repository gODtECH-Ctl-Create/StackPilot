use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

const STEWARD_SCHEMA_VERSION: u8 = 1;
const STEWARD_TOOL: &str = "gODtECH Steward";
const STEWARD_VERSION: u8 = 1;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StewardReport {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u8,
    pub tool: String,
    pub version: u8,
    pub health_score: u8,
    pub findings: Vec<StewardFinding>,
    #[serde(default, rename = "rulePacks")]
    pub rule_packs: Vec<StewardRulePack>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StewardFinding {
    pub id: String,
    pub rule: String,
    pub category: String,
    pub severity: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub line: Option<u32>,
    pub fixable: bool,
    pub confidence: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StewardRulePack {
    pub id: String,
    pub version: u32,
}

pub fn load(path: &Path) -> Result<StewardReport> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read Steward report {}", path.display()))?;
    let report = serde_json::from_str::<StewardReport>(&content)
        .with_context(|| format!("failed to parse Steward report {} as JSON", path.display()))?;

    validate(&report, path)?;
    Ok(report)
}

fn validate(report: &StewardReport, path: &Path) -> Result<()> {
    if report.schema_version != STEWARD_SCHEMA_VERSION {
        bail!(
            "unsupported Steward schemaVersion {} in {}; expected {}",
            report.schema_version,
            path.display(),
            STEWARD_SCHEMA_VERSION
        );
    }
    if report.tool != STEWARD_TOOL {
        bail!(
            "unsupported report tool {:?} in {}; expected {:?}",
            report.tool,
            path.display(),
            STEWARD_TOOL
        );
    }
    if report.version != STEWARD_VERSION {
        bail!(
            "unsupported Steward report version {} in {}; expected {}",
            report.version,
            path.display(),
            STEWARD_VERSION
        );
    }
    Ok(())
}

pub fn print_summary(report: &StewardReport) {
    println!("\nSteward repository health");
    println!("Health: {}/100", report.health_score);
    println!("Findings: {}", report.findings.len());
    println!(
        "Fixable findings: {}",
        report.findings.iter().filter(|finding| finding.fixable).count()
    );

    let mut severities = std::collections::BTreeMap::<&str, usize>::new();
    for finding in &report.findings {
        *severities.entry(finding.severity.as_str()).or_default() += 1;
    }
    if !severities.is_empty() {
        println!(
            "Severity: {}",
            severities
                .iter()
                .map(|(severity, count)| format!("{severity}={count}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if !report.rule_packs.is_empty() {
        let packs = report
            .rule_packs
            .iter()
            .map(|pack| format!("{}@{}", pack.id, pack.version))
            .collect::<Vec<_>>()
            .join(", ");
        println!("Rule packs: {packs}");
    }
}

#[cfg(test)]
mod tests {
    use super::{STEWART_SCHEMA_VERSION, STEWARD_TOOL, STEWARD_VERSION, load};
    use std::fs;

    #[test]
    fn accepts_versioned_steward_report() {
        let directory = tempfile::tempdir().expect("temp directory");
        let path = directory.path().join("steward.json");
        fs::write(
            &path,
            format!(
                r#"{{
                    "schemaVersion": {},
                    "tool": "{}",
                    "version": {},
                    "root": "/tmp/repo",
                    "scannedFiles": 3,
                    "textFiles": 2,
                    "findings": [],
                    "healthScore": 96,
                    "durationMs": 12,
                    "git": {{"isRepository": true}},
                    "categoryCounts": {{}},
                    "ruleCounts": {{}},
                    "rulePacks": [{{"id":"core","version":2}}]
                }}"#,
                STEWART_SCHEMA_VERSION, STEWARD_TOOL, STEWARD_VERSION
            ),
        )
        .expect("write report");

        let report = load(&path).expect("load report");
        assert_eq!(report.health_score, 96);
        assert_eq!(report.rule_packs[0].id, "core");
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let directory = tempfile::tempdir().expect("temp directory");
        let path = directory.path().join("steward.json");
        fs::write(
            &path,
            format!(
                r#"{{"schemaVersion":2,"tool":"{}","version":{},"healthScore":100,"findings":[]}}"#,
                STEWARD_TOOL, STEWARD_VERSION
            ),
        )
        .expect("write report");

        let error = load(&path).expect_err("schema should be rejected");
        assert!(error.to_string().contains("unsupported Steward schemaVersion"));
    }
}
