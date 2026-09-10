from pathlib import Path

path = Path('src/fix.rs')
text = path.read_text()

text = text.replace(
'''#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeferredFix {''',
'''#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedUpdate {
    pub path: PathBuf,
    pub description: String,
    original: String,
    content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeferredFix {''',
1,
)

text = text.replace(
'''    pub changes: Vec<PlannedChange>,
    pub deferred: Vec<DeferredFix>,''',
'''    pub changes: Vec<PlannedChange>,
    pub updates: Vec<PlannedUpdate>,
    pub deferred: Vec<DeferredFix>,''',
1,
)

text = text.replace(
'''        if !plan.changes.is_empty() {
            println!(''',
'''        if !plan.changes.is_empty() || !plan.updates.is_empty() {
            println!(''',
1,
)
text = text.replace(
'''    if plan.changes.is_empty() {
        println!("\\nNo safe automatic changes are currently required.");''',
'''    if plan.changes.is_empty() && plan.updates.is_empty() {
        println!("\\nNo safe automatic changes are currently required.");''',
1,
)

text = text.replace(
'''    let mut changes = Vec::new();

    plan_environment_fixes(&root, &report, &mut changes)?;
    let cloud = normalize_cloud(cloud)?;
    let deferred = plan_stack_fixes(&root, &report, recipes_dir, cloud, &mut changes)?;
    plan_metadata_fix(&root, &report, cloud, &mut changes)?;

    Ok(FixPlan {
        root,
        readiness_before,
        changes,
        deferred,
    })''',
'''    let mut changes = Vec::new();
    let mut updates = Vec::new();

    plan_environment_fixes(&root, &report, &mut changes)?;
    let cloud = normalize_cloud(cloud)?;
    let deferred = plan_stack_fixes(
        &root,
        &report,
        recipes_dir,
        cloud,
        &mut changes,
        &mut updates,
    )?;
    plan_metadata_fix(&root, &report, cloud, &mut changes)?;

    Ok(FixPlan {
        root,
        readiness_before,
        changes,
        updates,
        deferred,
    })''',
1,
)

text = text.replace(
'''    cloud: Option<&str>,
    changes: &mut Vec<PlannedChange>,
) -> Result<Vec<DeferredFix>> {''',
'''    cloud: Option<&str>,
    changes: &mut Vec<PlannedChange>,
    updates: &mut Vec<PlannedUpdate>,
) -> Result<Vec<DeferredFix>> {''',
1,
)

old_health = '''    if finding_status(report, "Health check") != Some(FindingStatus::Passed) {
        deferred.push(DeferredFix {
            control: "Health check",
            reason: "health remediation can touch application routing and needs a stack-specific source adapter"
                .to_string(),
        });
    }
'''
new_health = '''    if finding_status(report, "Health check") != Some(FindingStatus::Passed) {
        plan_health_fix(root, report, recipes_dir, changes, updates, &mut deferred)?;
    }
'''
if old_health not in text:
    raise SystemExit('health defer block not found')
text = text.replace(old_health, new_health, 1)

insert_at = 'fn plan_docker_fix(\n'
health_code = r'''fn plan_health_fix(
    root: &Path,
    report: &InspectionReport,
    recipes_dir: &Path,
    changes: &mut Vec<PlannedChange>,
    updates: &mut Vec<PlannedUpdate>,
    deferred: &mut Vec<DeferredFix>,
) -> Result<()> {
    let profile = match verified_stack_profile(root, report) {
        Ok(profile) => profile,
        Err(reason) => {
            deferred.push(DeferredFix {
                control: "Health check",
                reason: format!("health remediation requires a verified golden-path stack: {reason}"),
            });
            return Ok(());
        }
    };

    let outcome = match (profile.language, profile.framework) {
        ("Rust", "Axum") => plan_rust_health(root, updates),
        ("Go", "Chi") => plan_go_health(root, updates),
        ("TypeScript", "NestJS") => plan_nest_health(root, updates),
        ("Python", "FastAPI") => plan_fastapi_health(root, updates),
        ("Java", "Spring Boot") => {
            plan_spring_health(root, recipes_dir, &profile, changes)
        }
        ("C#", "ASP.NET Core") => plan_aspnet_health(root, updates),
        _ => Err("the detected framework does not have a health remediation adapter".to_string()),
    };

    if let Err(reason) = outcome {
        deferred.push(DeferredFix {
            control: "Health check",
            reason,
        });
    }

    Ok(())
}

fn plan_rust_health(
    root: &Path,
    updates: &mut Vec<PlannedUpdate>,
) -> std::result::Result<(), String> {
    let relative = Path::new("src/main.rs");
    let original = read_required_text(root, "src/main.rs")?;
    let marker = "Router::new()";
    if original.matches(marker).count() != 1 {
        return Err(
            "Axum health remediation requires exactly one Router::new() construction in src/main.rs"
                .to_string(),
        );
    }
    let replacement = "Router::new()\n        .route(\"/health\", axum::routing::get(|| async { axum::http::StatusCode::OK }))";
    let content = original.replacen(marker, replacement, 1);
    push_update(
        updates,
        relative,
        "add a verified Axum GET /health route without changing existing handlers",
        original,
        content,
    );
    Ok(())
}

fn plan_go_health(
    root: &Path,
    updates: &mut Vec<PlannedUpdate>,
) -> std::result::Result<(), String> {
    let relative = Path::new("main.go");
    let original = read_required_text(root, "main.go")?;
    if !original.contains("\"net/http\"") {
        return Err(
            "Chi health remediation requires main.go to already import net/http so StackPilot does not rewrite imports"
                .to_string(),
        );
    }
    let lines = original
        .lines()
        .filter(|line| line.contains(":= chi.NewRouter()"))
        .collect::<Vec<_>>();
    if lines.len() != 1 {
        return Err(
            "Chi health remediation requires exactly one '<router> := chi.NewRouter()' line in main.go"
                .to_string(),
        );
    }
    let line = lines[0];
    let variable = line
        .split_once(":=")
        .map(|(left, _)| left.trim())
        .filter(|name| valid_identifier(name))
        .ok_or_else(|| "Chi router variable could not be identified safely".to_string())?;
    let insertion = format!(
        "{line}\n\t{variable}.Get(\"/health\", func(writer http.ResponseWriter, _ *http.Request) {{\n\t\twriter.WriteHeader(http.StatusOK)\n\t}})"
    );
    let content = original.replacen(line, &insertion, 1);
    push_update(
        updates,
        relative,
        "add a verified Chi GET /health route using the existing router and net/http import",
        original,
        content,
    );
    Ok(())
}

fn plan_nest_health(
    root: &Path,
    updates: &mut Vec<PlannedUpdate>,
) -> std::result::Result<(), String> {
    let package = read_required_text(root, "package.json")?;
    if !package.contains("@nestjs/platform-express") {
        return Err(
            "NestJS health remediation requires @nestjs/platform-express so the startup middleware contract is known"
                .to_string(),
        );
    }
    let relative = Path::new("src/main.ts");
    let original = read_required_text(root, "src/main.ts")?;
    let lines = original
        .lines()
        .filter(|line| line.contains("const app = await NestFactory.create("))
        .collect::<Vec<_>>();
    if lines.len() != 1 {
        return Err(
            "NestJS health remediation requires exactly one NestFactory.create startup assignment in src/main.ts"
                .to_string(),
        );
    }
    let line = lines[0];
    let insertion = format!(
        "{line}\n  app.use('/health', (_request: any, response: any) => {{\n    response.status(200).json({{ status: 'ok' }});\n  }});"
    );
    let content = original.replacen(line, &insertion, 1);
    push_update(
        updates,
        relative,
        "add a verified NestJS /health middleware endpoint at application startup",
        original,
        content,
    );
    Ok(())
}

fn plan_fastapi_health(
    root: &Path,
    updates: &mut Vec<PlannedUpdate>,
) -> std::result::Result<(), String> {
    let relative = Path::new("app/main.py");
    let original = read_required_text(root, "app/main.py")?;
    if original.matches("app = FastAPI(").count() != 1 {
        return Err(
            "FastAPI health remediation requires exactly one application named app in app/main.py"
                .to_string(),
        );
    }
    let separator = if original.ends_with('\n') { "\n" } else { "\n\n" };
    let content = format!(
        "{original}{separator}@app.get(\"/health\")\ndef stackpilot_health() -> dict[str, str]:\n    return {{\"status\": \"ok\"}}\n"
    );
    push_update(
        updates,
        relative,
        "add a verified FastAPI GET /health route to the existing app",
        original,
        content,
    );
    Ok(())
}

fn plan_spring_health(
    root: &Path,
    recipes_dir: &Path,
    profile: &StackProfile,
    changes: &mut Vec<PlannedChange>,
) -> std::result::Result<(), String> {
    let application = Path::new("src/main/java/com/stackpilot/app/Application.java");
    let health = Path::new("src/main/java/com/stackpilot/app/HealthController.java");
    require_file(root, application.to_str().unwrap_or_default())?;
    if path_entry_exists(&root.join(health)) {
        return Err(
            "Spring Boot HealthController.java already exists but no health endpoint was detected; StackPilot will not overwrite it"
                .to_string(),
        );
    }
    if !recipes_dir.is_dir() {
        return Err(format!(
            "StackPilot recipes were not found at {}; pass --recipes-dir or reinstall StackPilot",
            recipes_dir.display()
        ));
    }
    first_symlink_ancestor(root, health)
        .map_err(|error| error.to_string())?
        .map_or(Ok(()), |path| {
            Err(format!(
                "the Spring health target path contains a symlink at {}; StackPilot will not write through it",
                path.display()
            ))
        })?;

    let spec = ProjectSpec::configured(
        profile.project_name.clone(),
        "Backend API".to_string(),
        profile.language.to_string(),
        profile.framework.to_string(),
        "None".to_string(),
        "None".to_string(),
        false,
        false,
        false,
    )
    .map_err(|error| error.to_string())?;
    let content = scaffold::render_destination(&spec, RECIPE_NAME, recipes_dir, health)
        .map_err(|error| error.to_string())?;
    changes.push(PlannedChange {
        path: health.to_path_buf(),
        kind: ChangeKind::Create,
        description: "add the verified Spring Boot GET /health controller in the standard package"
            .to_string(),
        content,
    });
    Ok(())
}

fn plan_aspnet_health(
    root: &Path,
    updates: &mut Vec<PlannedUpdate>,
) -> std::result::Result<(), String> {
    let relative = Path::new("Program.cs");
    let original = read_required_text(root, "Program.cs")?;
    let marker = "app.Run();";
    if original.matches(marker).count() != 1 {
        return Err(
            "ASP.NET Core health remediation requires exactly one app.Run(); in Program.cs"
                .to_string(),
        );
    }
    let replacement = "app.MapGet(\"/health\", () => Results.Ok(new { status = \"ok\" }));\n\napp.Run();";
    let content = original.replacen(marker, replacement, 1);
    push_update(
        updates,
        relative,
        "add a verified ASP.NET Core GET /health endpoint before app.Run()",
        original,
        content,
    );
    Ok(())
}

fn push_update(
    updates: &mut Vec<PlannedUpdate>,
    path: &Path,
    description: &str,
    original: String,
    content: String,
) {
    updates.push(PlannedUpdate {
        path: path.to_path_buf(),
        description: description.to_string(),
        original,
        content,
    });
}

fn valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

'''
if insert_at not in text:
    raise SystemExit('health insertion point missing')
text = text.replace(insert_at, health_code + insert_at, 1)

old_apply = '''fn apply_plan(plan: &FixPlan) -> Result<usize> {
    let mut applied = 0;

    for change in &plan.changes {'''
new_apply = '''fn apply_plan(plan: &FixPlan) -> Result<usize> {
    let mut applied = 0;

    for update in &plan.updates {
        ensure_safe_relative_path(&update.path)?;
        ensure_no_symlink_ancestors(&plan.root, &update.path)?;
        let target = plan.root.join(&update.path);
        let metadata = fs::symlink_metadata(&target)
            .with_context(|| format!("failed to inspect {}", target.display()))?;
        if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
            bail!("refusing to update non-regular file {}", target.display());
        }
        let current = fs::read_to_string(&target)
            .with_context(|| format!("failed to read {}", target.display()))?;
        if current != update.original {
            bail!(
                "refusing to update {} because it changed after the fix plan was created",
                target.display()
            );
        }
    }

    for change in &plan.changes {'''
if old_apply not in text:
    raise SystemExit('apply_plan start not found')
text = text.replace(old_apply, new_apply, 1)

old_apply_end = '''    }

    Ok(applied)
}

fn ensure_safe_relative_path'''
new_apply_end = '''    }

    for update in &plan.updates {
        let target = plan.root.join(&update.path);
        fs::write(&target, &update.content)
            .with_context(|| format!("failed to update {}", target.display()))?;
        applied += 1;
    }

    Ok(applied)
}

fn ensure_safe_relative_path'''
if old_apply_end not in text:
    raise SystemExit('apply_plan end not found')
text = text.replace(old_apply_end, new_apply_end, 1)

old_print = '''    if plan.changes.is_empty() {
        println!("  none");
    } else {
        for change in &plan.changes {
            println!(
                "{} {} {} — {}",
                change.kind.symbol(),
                change.kind.verb(),
                change.path.display(),
                change.description
            );
        }
    }
'''
new_print = '''    if plan.changes.is_empty() && plan.updates.is_empty() {
        println!("  none");
    } else {
        for change in &plan.changes {
            println!(
                "{} {} {} — {}",
                change.kind.symbol(),
                change.kind.verb(),
                change.path.display(),
                change.description
            );
        }
        for update in &plan.updates {
            println!("~ update {} — {}", update.path.display(), update.description);
        }
    }
'''
if old_print not in text:
    raise SystemExit('print plan block not found')
text = text.replace(old_print, new_print, 1)

# Strengthen Spring verified profile with the standard Application.java path, because
# the create-only health adapter relies on that package layout.
old_spring = '''        ("Java", "Spring Boot") => {
            require_file(root, "pom.xml")?;
            let pom = read_required_text(root, "pom.xml")?;'''
new_spring = '''        ("Java", "Spring Boot") => {
            require_file(root, "pom.xml")?;
            require_file(root, "src/main/java/com/stackpilot/app/Application.java")?;
            let pom = read_required_text(root, "pom.xml")?;'''
if old_spring not in text:
    raise SystemExit('Spring verified profile block not found')
text = text.replace(old_spring, new_spring, 1)

# Test imports.
text = text.replace(
'use super::{ChangeKind, apply_plan, plan_repository};',
'use super::{ChangeKind, apply_plan, plan_repository};',
1,
)

idx = text.rfind('\n}')
if idx == -1:
    raise SystemExit('test module end not found')
health_tests = r'''

    #[test]
    fn verified_go_health_update_is_planned_and_applied() {
        let repo = tempdir().expect("repository");
        fs::write(
            repo.path().join("go.mod"),
            "module example.com/payments\n\ngo 1.27\n\nrequire github.com/go-chi/chi/v5 v5.0.0\n",
        )
        .expect("go.mod");
        fs::write(
            repo.path().join("main.go"),
            "package main\n\nimport (\n\t\"net/http\"\n\t\"github.com/go-chi/chi/v5\"\n)\n\nfunc main() {\n\trouter := chi.NewRouter()\n\t_ = http.ListenAndServe(\":3000\", router)\n}\n",
        )
        .expect("main.go");

        let plan = plan_repository(repo.path(), Path::new("recipes"), None).expect("fix plan");
        assert!(plan.updates.iter().any(|update| update.path == Path::new("main.go")));
        assert!(!plan.deferred.iter().any(|fix| fix.control == "Health check"));

        apply_plan(&plan).expect("apply plan");
        let main = fs::read_to_string(repo.path().join("main.go")).expect("main.go");
        assert!(main.contains("router.Get(\"/health\""));
    }

    #[test]
    fn source_update_refuses_changed_file_after_plan() {
        let repo = tempdir().expect("repository");
        fs::write(
            repo.path().join("go.mod"),
            "module example.com/payments\n\ngo 1.27\n\nrequire github.com/go-chi/chi/v5 v5.0.0\n",
        )
        .expect("go.mod");
        fs::write(
            repo.path().join("main.go"),
            "package main\n\nimport (\n\t\"net/http\"\n\t\"github.com/go-chi/chi/v5\"\n)\n\nfunc main() {\n\trouter := chi.NewRouter()\n\t_ = http.ListenAndServe(\":3000\", router)\n}\n",
        )
        .expect("main.go");

        let plan = plan_repository(repo.path(), Path::new("recipes"), None).expect("fix plan");
        fs::write(repo.path().join("main.go"), "package main\n// developer changed this\n")
            .expect("change source after planning");

        let error = apply_plan(&plan).expect_err("changed source must be refused");
        assert!(error.to_string().contains("changed after the fix plan was created"));
        assert!(!repo.path().join(".env.example").exists());
    }

    #[test]
    fn ambiguous_go_router_keeps_health_deferred() {
        let repo = tempdir().expect("repository");
        fs::write(
            repo.path().join("go.mod"),
            "module example.com/payments\n\ngo 1.27\n\nrequire github.com/go-chi/chi/v5 v5.0.0\n",
        )
        .expect("go.mod");
        fs::write(
            repo.path().join("main.go"),
            "package main\n\nimport (\n\t\"net/http\"\n\t\"github.com/go-chi/chi/v5\"\n)\n\nfunc main() {\n\ta := chi.NewRouter()\n\tb := chi.NewRouter()\n\t_, _ = a, b\n}\n",
        )
        .expect("main.go");

        let plan = plan_repository(repo.path(), Path::new("recipes"), None).expect("fix plan");
        assert!(!plan.updates.iter().any(|update| update.path == Path::new("main.go")));
        assert!(plan.deferred.iter().any(|fix| fix.control == "Health check"));
    }
'''
text = text[:idx] + health_tests + text[idx:]
path.write_text(text)

# Documentation.
doc = Path('docs/remediation.md')
doc_text = doc.read_text()
doc_text += r'''

## Verified health-check remediation

When `stackpilot inspect` reports that a supported backend has no health endpoint, `stackpilot fix` can now plan a real `GET /health` endpoint for verified Axum, Chi, NestJS, FastAPI, Spring Boot, and ASP.NET Core layouts. It does not satisfy readiness by adding a comment or configuration marker: the remediation changes application routing or, for the standard Spring Boot layout, creates the tested health controller.

Health source edits use compare-before-write protection. StackPilot stores the exact source text used to create the preview and, during `--apply`, refuses the update if that file changed in the meantime. Ambiguous router/application startup patterns, unsupported package layouts, symlinked paths, or unknown stacks remain deferred rather than guessed.
'''
doc.write_text(doc_text)
