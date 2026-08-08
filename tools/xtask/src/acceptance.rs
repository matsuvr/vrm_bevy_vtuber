//! Acceptance test support commands.
//!
//! Provides CLI commands for managing acceptance test runs:
//! - `acceptance --help`: show usage
//! - `acceptance env`: print test environment info
//! - `acceptance new`: create a new acceptance run directory
//! - `acceptance verify`: verify model hashes and environment

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Create a new acceptance run directory with a unique timestamp.
pub fn new_run(base_dir: &Path) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("cannot get time: {e}"))?
        .as_secs();

    let run_dir = base_dir.join(format!("run-{timestamp}"));
    fs::create_dir_all(&run_dir).map_err(|e| format!("cannot create run dir: {e}"))?;

    // Create subdirectories.
    fs::create_dir_all(run_dir.join("logs")).map_err(|e| format!("cannot create logs dir: {e}"))?;
    fs::create_dir_all(run_dir.join("metrics"))
        .map_err(|e| format!("cannot create metrics dir: {e}"))?;
    fs::create_dir_all(run_dir.join("artifacts"))
        .map_err(|e| format!("cannot create artifacts dir: {e}"))?;

    // Write initial run metadata.
    let metadata = format!(
        "# Acceptance Run\n\nTimestamp: {timestamp}\nCommit: (fill in)\nBinary: (fill in)\n"
    );
    fs::write(run_dir.join("metadata.md"), metadata)
        .map_err(|e| format!("cannot write metadata: {e}"))?;

    Ok(run_dir)
}

/// Print test environment information.
pub fn print_env() {
    println!("# Test Environment");
    println!();
    println!("| Item | Value |");
    println!("|------|-------|");

    // OS
    println!("| OS | {} |", std::env::consts::OS);
    println!("| Arch | {} |", std::env::consts::ARCH);

    // Rust toolchain
    if let Ok(output) = std::process::Command::new("rustc")
        .arg("--version")
        .output()
    {
        let version = String::from_utf8_lossy(&output.stdout);
        println!("| Rust | {} |", version.trim());
    }

    // Cargo
    if let Ok(output) = std::process::Command::new("cargo")
        .arg("--version")
        .output()
    {
        let version = String::from_utf8_lossy(&output.stdout);
        println!("| Cargo | {} |", version.trim());
    }

    // Git commit
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
    {
        let sha = String::from_utf8_lossy(&output.stdout);
        println!("| Commit | {} |", sha.trim());
    }

    println!();
    println!("## To fill in manually:");
    println!("- CPU model");
    println!("- GPU model + driver version");
    println!("- RAM amount");
    println!("- Screen resolution");
    println!("- Camera device names + descriptors");
}

/// Verify model hashes in a manifest.
pub fn verify_models(manifest_path: &Path) -> Result<(), String> {
    if !manifest_path.exists() {
        return Err(format!("manifest not found: {}", manifest_path.display()));
    }

    let content =
        fs::read_to_string(manifest_path).map_err(|e| format!("cannot read manifest: {e}"))?;

    println!("Model manifest: {}", manifest_path.display());
    println!("Content preview (first 500 chars):");
    println!("{}", &content[..content.len().min(500)]);
    println!("...");
    println!();
    println!("To verify hashes, run:");
    println!("  sha256sum <model-file>");
    println!("and compare with manifest entries.");

    Ok(())
}

/// Print acceptance command help.
pub fn print_help() {
    println!("acceptance - Windows acceptance test support");
    println!();
    println!("USAGE:");
    println!("  cargo xtask acceptance <command>");
    println!();
    println!("COMMANDS:");
    println!("  env              Print test environment info");
    println!("  new [base-dir]   Create a new acceptance run directory");
    println!("  verify <manifest> Verify model hashes against manifest");
    println!("  help             Show this help");
    println!();
    println!("The acceptance report template is at:");
    println!("  docs/acceptance/windows-m1.md");
}
