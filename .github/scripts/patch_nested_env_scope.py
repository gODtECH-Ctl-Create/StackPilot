from pathlib import Path

path = Path('src/fix.rs')
text = path.read_text()

old_call = '    plan_environment_fixes(&root, &report, &mut changes)?;'
new_call = '    plan_environment_fixes(&root, &report, &mut changes, &mut deferred)?;'
if old_call not in text:
    raise SystemExit('environment planner call pattern not found')
text = text.replace(old_call, new_call, 1)

old_sig = '''fn plan_environment_fixes(
    root: &Path,
    report: &InspectionReport,
    changes: &mut Vec<PlannedChange>,
) -> Result<()> {
    if finding_status(report, "Environment config") == Some(FindingStatus::Passed) {
        return Ok(());
    }
'''
new_sig = '''fn plan_environment_fixes(
    root: &Path,
    report: &InspectionReport,
    changes: &mut Vec<PlannedChange>,
    deferred: &mut Vec<DeferredFix>,
) -> Result<()> {
    if finding_status(report, "Environment config") == Some(FindingStatus::Passed) {
        return Ok(());
    }

    if report.repository_root != root {
        deferred.push(DeferredFix {
            control: "Environment config",
            reason: format!(
                "environment configuration is inherited from repository root {}; run `stackpilot fix {}` from the repository root to review repository-scoped environment remediation",
                report.repository_root.display(),
                report.repository_root.display()
            ),
        });
        return Ok(());
    }
'''
if old_sig not in text:
    raise SystemExit('environment planner signature pattern not found')
text = text.replace(old_sig, new_sig, 1)

path.write_text(text)
