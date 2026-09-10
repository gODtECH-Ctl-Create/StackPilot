from pathlib import Path

p = Path('src/main.rs')
s = p.read_text()
old = '''        /// Directory that contains StackPilot recipes used by verified stack adapters.
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,

        /// Write the planned safe changes. Without this flag, fix is a dry-run preview.
        #[arg(long)]
        apply: bool,'''
new = '''        /// Directory that contains StackPilot recipes used by verified stack adapters.
        #[arg(long, default_value = "recipes")]
        recipes_dir: PathBuf,

        /// Explicit cloud target for Terraform remediation: AWS, Azure, or GCP.
        #[arg(long, value_name = "CLOUD")]
        cloud: Option<String>,

        /// Write the planned safe changes. Without this flag, fix is a dry-run preview.
        #[arg(long)]
        apply: bool,'''
if old not in s:
    raise SystemExit('main Fix args pattern not found')
s = s.replace(old, new, 1)
old = '''        Commands::Fix {
            path,
            recipes_dir,
            apply,
        } => {
            let recipes_dir = resolve_recipes_dir(recipes_dir);
            fix::run(&path, &recipes_dir, apply)?;
        }'''
new = '''        Commands::Fix {
            path,
            recipes_dir,
            cloud,
            apply,
        } => {
            let recipes_dir = resolve_recipes_dir(recipes_dir);
            fix::run(&path, &recipes_dir, cloud.as_deref(), apply)?;
        }'''
if old not in s:
    raise SystemExit('main Fix match pattern not found')
p.write_text(s.replace(old, new, 1))

p = Path('src/fix.rs')
s = p.read_text()
s = s.replace(
    'pub fn run(root: &Path, recipes_dir: &Path, apply: bool) -> Result<()> {\n    let plan = plan_repository(root, recipes_dir)?;',
    'pub fn run(root: &Path, recipes_dir: &Path, cloud: Option<&str>, apply: bool) -> Result<()> {\n    let plan = plan_repository(root, recipes_dir, cloud)?;',
    1,
)
s = s.replace(
    'pub fn plan_repository(root: &Path, recipes_dir: &Path) -> Result<FixPlan> {',
    'pub fn plan_repository(root: &Path, recipes_dir: &Path, cloud: Option<&str>) -> Result<FixPlan> {',
    1,
)
s = s.replace(
    '    let deferred = plan_stack_fixes(&root, &report, recipes_dir, &mut changes)?;\n    plan_metadata_fix(&root, &report, &mut changes)?;',
    '    let cloud = normalize_cloud(cloud)?;\n    let deferred = plan_stack_fixes(&root, &report, recipes_dir, cloud, &mut changes)?;\n    plan_metadata_fix(&root, &report, cloud, &mut changes)?;',
    1,
)
s = s.replace(
    '    recipes_dir: &Path,\n    changes: &mut Vec<PlannedChange>,\n) -> Result<Vec<DeferredFix>> {',
    '    recipes_dir: &Path,\n    cloud: Option<&str>,\n    changes: &mut Vec<PlannedChange>,\n) -> Result<Vec<DeferredFix>> {',
    1,
)
old_tf = '''    if finding_status(report, "Terraform") != Some(FindingStatus::Passed) {
        deferred.push(DeferredFix {
            control: "Terraform",
            reason: "infrastructure generation needs an explicit cloud/deployment target rather than guessing"
                .to_string(),
        });
    }'''
new_tf = '''    if finding_status(report, "Terraform") != Some(FindingStatus::Passed) {
        match cloud {
            Some(cloud) if recipes_dir.is_dir() => {
                plan_terraform_fix(root, report, recipes_dir, cloud, changes, &mut deferred)?;
            }
            Some(_) => deferred.push(DeferredFix {
                control: "Terraform",
                reason: format!(
                    "StackPilot recipes were not found at {}; pass --recipes-dir or reinstall StackPilot",
                    recipes_dir.display()
                ),
            }),
            None => deferred.push(DeferredFix {
                control: "Terraform",
                reason: "infrastructure generation requires explicit intent; pass --cloud AWS, --cloud Azure, or --cloud GCP"
                    .to_string(),
            }),
        }
    }'''
if old_tf not in s:
    raise SystemExit('terraform defer block not found')
s = s.replace(old_tf, new_tf, 1)

insert_before = 'fn verified_stack_profile(\n'
terraform_fn = '''fn plan_terraform_fix(
    root: &Path,
    report: &InspectionReport,
    recipes_dir: &Path,
    cloud: &str,
    changes: &mut Vec<PlannedChange>,
    deferred: &mut Vec<DeferredFix>,
) -> Result<()> {
    let targets = [
        Path::new("infra/terraform/main.tf"),
        Path::new("infra/terraform/variables.tf"),
        Path::new("infra/terraform/README.md"),
    ];

    if targets.iter().any(|path| path_entry_exists(&root.join(path))) {
        deferred.push(DeferredFix {
            control: "Terraform",
            reason: "a Terraform foundation already exists but was not recognized; StackPilot will not overwrite or replace it"
                .to_string(),
        });
        return Ok(());
    }

    if let Some(path) = first_symlink_ancestor(root, Path::new("infra/terraform/main.tf"))? {
        deferred.push(DeferredFix {
            control: "Terraform",
            reason: format!(
                "the Terraform target path contains a symlink at {}; StackPilot will not write through symlinked directories",
                path.display()
            ),
        });
        return Ok(());
    }

    let profile = verified_stack_profile(root, report).ok();
    let (language, framework, project_name) = match profile {
        Some(profile) => (profile.language, profile.framework, profile.project_name),
        None => ("Generic", "None", repository_name(root)),
    };

    let spec = ProjectSpec::configured(
        project_name,
        "Generic".to_string(),
        language.to_string(),
        framework.to_string(),
        "None".to_string(),
        cloud.to_string(),
        false,
        false,
        true,
    )?;

    for (path, description) in [
        (
            Path::new("infra/terraform/main.tf"),
            "generate the explicit-cloud Terraform provider foundation",
        ),
        (
            Path::new("infra/terraform/variables.tf"),
            "generate Terraform variables for the selected cloud",
        ),
        (
            Path::new("infra/terraform/README.md"),
            "document the generated Terraform foundation and next deployment steps",
        ),
    ] {
        changes.push(PlannedChange {
            path: path.to_path_buf(),
            kind: ChangeKind::Create,
            description: description.to_string(),
            content: scaffold::render_destination(&spec, RECIPE_NAME, recipes_dir, path)?,
        });
    }

    Ok(())
}

fn normalize_cloud(cloud: Option<&str>) -> Result<Option<&'static str>> {
    match cloud {
        None => Ok(None),
        Some(value) if value.eq_ignore_ascii_case("AWS") => Ok(Some("AWS")),
        Some(value) if value.eq_ignore_ascii_case("Azure") => Ok(Some("Azure")),
        Some(value) if value.eq_ignore_ascii_case("GCP") => Ok(Some("GCP")),
        Some(value) => bail!("unsupported cloud '{value}'; choose one of: AWS, Azure, GCP"),
    }
}

'''
if insert_before not in s:
    raise SystemExit('verified_stack_profile insertion point missing')
s = s.replace(insert_before, terraform_fn + insert_before, 1)

s = s.replace(
    'fn plan_metadata_fix(\n    root: &Path,\n    report: &InspectionReport,\n    changes: &mut Vec<PlannedChange>,\n) -> Result<()> {',
    'fn plan_metadata_fix(\n    root: &Path,\n    report: &InspectionReport,\n    cloud: Option<&str>,\n    changes: &mut Vec<PlannedChange>,\n) -> Result<()> {',
    1,
)
s = s.replace(
    '        content: stackpilot_metadata(root, report, changes),',
    '        content: stackpilot_metadata(root, report, cloud, changes),',
    1,
)
s = s.replace(
    'fn stackpilot_metadata(\n    root: &Path,\n    report: &InspectionReport,\n    changes: &[PlannedChange],\n) -> String {',
    'fn stackpilot_metadata(\n    root: &Path,\n    report: &InspectionReport,\n    cloud: Option<&str>,\n    changes: &[PlannedChange],\n) -> String {',
    1,
)
s = s.replace(
    '    let terraform = finding_status(report, "Terraform") == Some(FindingStatus::Passed);',
    '    let terraform = finding_status(report, "Terraform") == Some(FindingStatus::Passed)\n        || planned_create(changes, "infra/terraform/main.tf");\n    let cloud = cloud.unwrap_or("None");',
    1,
)
s = s.replace('cloud = \\"None\\"', 'cloud = \\"{cloud}\\"', 1)
s = s.replace('plan_repository(repo.path(), Path::new("recipes"))', 'plan_repository(repo.path(), Path::new("recipes"), None)')

idx = s.rfind('\n}')
tests = '''

    #[test]
    fn explicit_aws_cloud_plans_terraform_foundation() {
        let repo = tempdir().expect("repository");
        fs::write(repo.path().join("README.md"), "# demo\\n").expect("readme");
        let plan = plan_repository(repo.path(), Path::new("recipes"), Some("aws"))
            .expect("fix plan");
        for path in [
            "infra/terraform/main.tf",
            "infra/terraform/variables.tf",
            "infra/terraform/README.md",
        ] {
            assert!(plan.changes.iter().any(|change| change.path == Path::new(path)));
        }
        assert!(!plan.deferred.iter().any(|fix| fix.control == "Terraform"));
        apply_plan(&plan).expect("apply plan");
        let main = fs::read_to_string(repo.path().join("infra/terraform/main.tf"))
            .expect("terraform main");
        assert!(main.contains("provider \\\"aws\\\""));
    }

    #[test]
    fn terraform_stays_deferred_without_explicit_cloud() {
        let repo = tempdir().expect("repository");
        fs::write(repo.path().join("README.md"), "# demo\\n").expect("readme");
        let plan = plan_repository(repo.path(), Path::new("recipes"), None).expect("fix plan");
        assert!(plan.deferred.iter().any(|fix| {
            fix.control == "Terraform" && fix.reason.contains("--cloud AWS")
        }));
    }

    #[test]
    fn rejects_unsupported_cloud_for_terraform_fix() {
        let repo = tempdir().expect("repository");
        let result = plan_repository(repo.path(), Path::new("recipes"), Some("DigitalOcean"));
        assert!(result.is_err());
    }
'''
s = s[:idx] + tests + s[idx:]
p.write_text(s)

p = Path('.github/workflows/ci.yml')
s = p.read_text()
anchor = '      - name: Smoke test installed recipe resolution\n'
smoke = '''      - name: Smoke test explicit-cloud Terraform remediation
        run: |
          rm -rf /tmp/stackpilot-terraform-fix
          mkdir -p /tmp/stackpilot-terraform-fix
          printf '# demo\\n' > /tmp/stackpilot-terraform-fix/README.md

          ./target/debug/stackpilot fix /tmp/stackpilot-terraform-fix --cloud AWS > terraform-preview.txt
          grep -q 'create infra/terraform/main.tf' terraform-preview.txt
          grep -q 'create infra/terraform/variables.tf' terraform-preview.txt
          test ! -f /tmp/stackpilot-terraform-fix/infra/terraform/main.tf

          ./target/debug/stackpilot fix /tmp/stackpilot-terraform-fix --cloud AWS --apply > terraform-apply.txt
          test -f /tmp/stackpilot-terraform-fix/infra/terraform/main.tf
          test -f /tmp/stackpilot-terraform-fix/infra/terraform/variables.tf
          grep -q 'provider "aws"' /tmp/stackpilot-terraform-fix/infra/terraform/main.tf
          grep -q 'cloud = "AWS"' /tmp/stackpilot-terraform-fix/.stackpilot.toml
          grep -q 'terraform = true' /tmp/stackpilot-terraform-fix/.stackpilot.toml

'''
if anchor not in s:
    raise SystemExit('CI insertion anchor not found')
p.write_text(s.replace(anchor, smoke + anchor, 1))

p = Path('docs/remediation.md')
s = p.read_text()
s += '''
## Explicit-cloud Terraform remediation

Terraform remains opt-in for existing repositories. StackPilot will not guess a cloud target. Pass one of:

```bash
stackpilot fix --cloud AWS
stackpilot fix --cloud Azure
stackpilot fix --cloud GCP
```

The command still previews by default. Add `--apply` only after reviewing the plan. When Terraform is missing and the selected cloud is valid, StackPilot can create `infra/terraform/main.tf`, `variables.tf`, and `README.md` from the same tested recipe templates used for greenfield projects. Existing or unrecognized Terraform files are never overwritten. Symlinked target paths are refused.
'''
p.write_text(s)
