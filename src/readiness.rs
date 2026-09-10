use std::path::Path;

use anyhow::{Result, bail};

use crate::inspect::{self, Finding, FindingStatus, InspectionReport};

pub const READINESS_MODEL: &str = "readiness-v1";

#[derive(Debug, Clone, Copy)]
struct Control {
    name: &'static str,
    category: &'static str,
    weight: u8,
}

const CONTROLS: &[Control] = &[
    Control {
        name: "Language",
        category: "Runtime",
        weight: 5,
    },
    Control {
        name: "Framework",
        category: "Runtime",
        weight: 5,
    },
    Control {
        name: "Docker",
        category: "Runtime",
        weight: 15,
    },
    Control {
        name: "CI/CD",
        category: "Delivery",
        weight: 20,
    },
    Control {
        name: "Terraform",
        category: "Infrastructure",
        weight: 15,
    },
    Control {
        name: "Environment config",
        category: "Security",
        weight: 25,
    },
    Control {
        name: "Health check",
        category: "Operability",
        weight: 15,
    },
];

const CATEGORY_ORDER: &[&str] = &[
    "Runtime",
    "Delivery",
    "Infrastructure",
    "Security",
    "Operability",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryScore {
    pub name: &'static str,
    pub earned: u8,
    pub maximum: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessScore {
    pub model: &'static str,
    pub total: u8,
    pub categories: Vec<CategoryScore>,
}

pub fn run(root: &Path, fail_below: Option<u8>) -> Result<()> {
    if fail_below.is_some_and(|threshold| threshold > 100) {
        bail!("--fail-below must be between 0 and 100");
    }

    let report = inspect::inspect_repository(root)?;
    let readiness = score(&report);
    print_report(&report, &readiness, fail_below);

    if let Some(threshold) = fail_below
        && readiness.total < threshold
    {
        bail!(
            "repository readiness score {}/100 is below required threshold {}",
            readiness.total,
            threshold
        );
    }

    Ok(())
}

pub fn score(report: &InspectionReport) -> ReadinessScore {
    let categories = CATEGORY_ORDER
        .iter()
        .map(|category| {
            let controls: Vec<Control> = CONTROLS
                .iter()
                .copied()
                .filter(|control| control.category == *category)
                .collect();

            let maximum = controls.iter().map(|control| control.weight).sum();
            let earned = controls
                .iter()
                .map(|control| score_control(report, *control))
                .sum();

            CategoryScore {
                name: category,
                earned,
                maximum,
            }
        })
        .collect::<Vec<_>>();

    let total = categories.iter().map(|category| category.earned).sum();

    ReadinessScore {
        model: READINESS_MODEL,
        total,
        categories,
    }
}

fn score_control(report: &InspectionReport, control: Control) -> u8 {
    let Some(finding) = report
        .findings
        .iter()
        .find(|finding| finding.name == control.name)
    else {
        return 0;
    };

    match finding.status {
        FindingStatus::Passed => control.weight,
        FindingStatus::Warning => control.weight / 2,
        FindingStatus::Missing => 0,
    }
}

fn print_report(report: &InspectionReport, readiness: &ReadinessScore, fail_below: Option<u8>) {
    println!("StackPilot Readiness: {}/100", readiness.total);
    println!("Model: {}", readiness.model);
    println!("Repository: {}", report.root.display());
    println!(
        "Languages: {}",
        display_values(&report.languages, "not detected")
    );
    println!(
        "Frameworks: {}",
        display_values(&report.frameworks, "not detected")
    );

    println!("\nCategory scores");
    for category in &readiness.categories {
        println!(
            "{:<14} {:>2}/{:<2}",
            category.name, category.earned, category.maximum
        );
    }

    println!("\nControls");
    for control in CONTROLS {
        let finding = report
            .findings
            .iter()
            .find(|finding| finding.name == control.name);
        print_control(finding, *control);
    }

    let informational: Vec<&Finding> = report
        .findings
        .iter()
        .filter(|finding| !CONTROLS.iter().any(|control| control.name == finding.name))
        .collect();

    if !informational.is_empty() {
        println!("\nAdditional findings");
        for finding in informational {
            println!(
                "{} {} — {}",
                status_symbol(finding.status),
                finding.name,
                finding.detail
            );
        }
    }

    let recommendations: Vec<&str> = report
        .findings
        .iter()
        .filter_map(|finding| finding.recommendation.as_deref())
        .collect();

    if recommendations.is_empty() {
        println!("\nRecommendations: none from the current readiness model");
    } else {
        println!("\nRecommendations: {}", recommendations.len());
        for recommendation in recommendations {
            println!("  - {recommendation}");
        }
    }

    if let Some(threshold) = fail_below {
        println!("\nRequired threshold: {threshold}");
        if readiness.total >= threshold {
            println!("✓ Repository readiness meets the required threshold.");
        } else {
            println!("✗ Repository readiness is below the required threshold.");
        }
    }
}

fn print_control(finding: Option<&Finding>, control: Control) {
    let (status, detail, earned) = match finding {
        Some(finding) => (
            finding.status,
            finding.detail.as_str(),
            match finding.status {
                FindingStatus::Passed => control.weight,
                FindingStatus::Warning => control.weight / 2,
                FindingStatus::Missing => 0,
            },
        ),
        None => (FindingStatus::Missing, "Control was not evaluated", 0),
    };

    println!(
        "{} [{:<14}] {} {}/{} — {}",
        status_symbol(status),
        control.category,
        control.name,
        earned,
        control.weight,
        detail
    );
}

fn display_values(values: &[String], fallback: &str) -> String {
    if values.is_empty() {
        fallback.to_string()
    } else {
        values.join(", ")
    }
}

fn status_symbol(status: FindingStatus) -> &'static str {
    match status {
        FindingStatus::Passed => "✓",
        FindingStatus::Warning => "!",
        FindingStatus::Missing => "✗",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{READINESS_MODEL, score};
    use crate::inspect::{Finding, FindingStatus, InspectionReport};

    fn report_with_statuses(statuses: &[(&'static str, FindingStatus)]) -> InspectionReport {
        InspectionReport {
            root: PathBuf::from("/tmp/example"),
            languages: vec!["TypeScript".to_string()],
            frameworks: vec!["NestJS".to_string()],
            findings: statuses
                .iter()
                .map(|(name, status)| Finding {
                    category: "Test",
                    name,
                    status: *status,
                    detail: "test finding".to_string(),
                    recommendation: None,
                })
                .collect(),
        }
    }

    #[test]
    fn full_readiness_scores_100() {
        let report = report_with_statuses(&[
            ("Language", FindingStatus::Passed),
            ("Framework", FindingStatus::Passed),
            ("Docker", FindingStatus::Passed),
            ("CI/CD", FindingStatus::Passed),
            ("Terraform", FindingStatus::Passed),
            ("Environment config", FindingStatus::Passed),
            ("Health check", FindingStatus::Passed),
        ]);

        let readiness = score(&report);

        assert_eq!(readiness.model, READINESS_MODEL);
        assert_eq!(readiness.total, 100);
        assert_eq!(
            readiness
                .categories
                .iter()
                .map(|category| category.maximum)
                .sum::<u8>(),
            100
        );
    }

    #[test]
    fn warning_receives_half_credit() {
        let report = report_with_statuses(&[
            ("Language", FindingStatus::Passed),
            ("Framework", FindingStatus::Warning),
            ("Docker", FindingStatus::Passed),
            ("CI/CD", FindingStatus::Passed),
            ("Terraform", FindingStatus::Missing),
            ("Environment config", FindingStatus::Warning),
            ("Health check", FindingStatus::Passed),
        ]);

        let readiness = score(&report);

        assert_eq!(readiness.total, 69);
    }

    #[test]
    fn missing_control_receives_zero_credit() {
        let report = report_with_statuses(&[("Language", FindingStatus::Passed)]);

        let readiness = score(&report);

        assert_eq!(readiness.total, 5);
    }
}
