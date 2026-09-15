use std::{fs, path::Path};

use anyhow::{Context, Result};

pub fn sync_after_fix(root: &Path, cloud: Option<&str>, apply: bool) -> Result<()> {
    let Some(cloud) = normalize_cloud(cloud) else {
        return Ok(());
    };

    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve repository path {}", root.display()))?;
    let metadata_path = root.join(".stackpilot.toml");
    if !metadata_path.exists() {
        return Ok(());
    }

    let metadata = fs::symlink_metadata(&metadata_path)
        .with_context(|| format!("failed to inspect {}", metadata_path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        println!(
            "\n! StackPilot metadata synchronization skipped because {} is not a regular file",
            metadata_path.display()
        );
        return Ok(());
    }

    let original = fs::read_to_string(&metadata_path)
        .with_context(|| format!("failed to read {}", metadata_path.display()))?;
    let Some(content) = synchronized_content(&original, cloud) else {
        println!(
            "\n! StackPilot metadata synchronization skipped because .stackpilot.toml does not contain the expected [project].cloud and [features].terraform keys"
        );
        return Ok(());
    };

    if content == original {
        return Ok(());
    }

    let terraform_exists = root.join("infra/terraform/main.tf").is_file();
    if !apply {
        println!("\nStackPilot metadata synchronization");
        println!(
            "~ update .stackpilot.toml — synchronize explicit cloud intent ({cloud}) and Terraform feature metadata"
        );
        if !terraform_exists {
            println!("  conditional on Terraform remediation being applied successfully");
        }
        return Ok(());
    }

    if !terraform_exists {
        return Ok(());
    }

    fs::write(&metadata_path, content)
        .with_context(|| format!("failed to update {}", metadata_path.display()))?;
    println!("\n✓ StackPilot metadata synchronized: cloud={cloud}, terraform=true");
    Ok(())
}

fn normalize_cloud(cloud: Option<&str>) -> Option<&'static str> {
    match cloud {
        Some(value) if value.eq_ignore_ascii_case("AWS") => Some("AWS"),
        Some(value) if value.eq_ignore_ascii_case("Azure") => Some("Azure"),
        Some(value) if value.eq_ignore_ascii_case("GCP") => Some("GCP"),
        _ => None,
    }
}

fn synchronized_content(content: &str, cloud: &str) -> Option<String> {
    let content = replace_section_key(content, "project", "cloud", &format!("\"{cloud}\""))?;
    replace_section_key(&content, "features", "terraform", "true")
}

fn replace_section_key(content: &str, section: &str, key: &str, value: &str) -> Option<String> {
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let had_trailing_newline = content.ends_with('\n');
    let target_section = format!("[{section}]");
    let target_key = format!("{key} =");
    let mut current_section = "";
    let mut replaced = false;
    let mut lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed;
        }

        if current_section == target_section && trimmed.starts_with(&target_key) {
            let indent_len = line.len() - line.trim_start().len();
            let indent = &line[..indent_len];
            lines.push(format!("{indent}{key} = {value}"));
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }

    if !replaced {
        return None;
    }

    let mut result = lines.join(newline);
    if had_trailing_newline {
        result.push_str(newline);
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{sync_after_fix, synchronized_content};

    const METADATA: &str = "version = 1\n\n[project]\nname = \"demo\"\ncloud = \"None\"\n\n[features]\ndocker = true\nci = true\nterraform = false\n\n[stackpilot]\nrecipe = \"adopted\"\nmanaged = false\n";

    #[test]
    fn synchronizes_cloud_and_terraform_without_rewriting_other_metadata() {
        let updated = synchronized_content(METADATA, "AWS").expect("metadata update");
        assert!(updated.contains("name = \"demo\""));
        assert!(updated.contains("cloud = \"AWS\""));
        assert!(updated.contains("terraform = true"));
        assert!(updated.contains("managed = false"));
    }

    #[test]
    fn apply_updates_adopted_metadata_after_terraform_exists() {
        let repo = tempdir().expect("repository");
        fs::write(repo.path().join(".stackpilot.toml"), METADATA).expect("metadata");
        fs::create_dir_all(repo.path().join("infra/terraform")).expect("terraform directory");
        fs::write(repo.path().join("infra/terraform/main.tf"), "terraform {}\n")
            .expect("terraform main");

        sync_after_fix(repo.path(), Some("aws"), true).expect("sync metadata");

        let updated = fs::read_to_string(repo.path().join(".stackpilot.toml")).expect("metadata");
        assert!(updated.contains("cloud = \"AWS\""));
        assert!(updated.contains("terraform = true"));
    }

    #[test]
    fn preview_never_modifies_metadata() {
        let repo = tempdir().expect("repository");
        fs::write(repo.path().join(".stackpilot.toml"), METADATA).expect("metadata");

        sync_after_fix(repo.path(), Some("AWS"), false).expect("preview metadata");

        assert_eq!(
            fs::read_to_string(repo.path().join(".stackpilot.toml")).expect("metadata"),
            METADATA
        );
    }
}
