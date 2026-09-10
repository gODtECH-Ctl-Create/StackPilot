use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};

pub fn is_available() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub fn init_repository(directory: &Path) -> Result<()> {
    if directory.join(".git").exists() {
        return Ok(());
    }

    let init = Command::new("git")
        .arg("init")
        .current_dir(directory)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .context("Git is required unless --no-git is supplied")?;

    if !init.status.success() {
        bail!(
            "failed to initialize Git repository: {}",
            String::from_utf8_lossy(&init.stderr).trim()
        );
    }

    let branch = Command::new("git")
        .args(["branch", "-M", "main"])
        .current_dir(directory)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .context("failed to configure Git default branch")?;

    if !branch.status.success() {
        bail!(
            "failed to set Git default branch: {}",
            String::from_utf8_lossy(&branch.stderr).trim()
        );
    }

    Ok(())
}
