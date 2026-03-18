use std::process::Command;

fn main() {
    // Embed git commit hash at build time
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();

    let hash = match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        }
        _ => "unknown".to_string(),
    };

    println!("cargo:rustc-env=GIT_HASH={hash}");
    // Rebuild if git HEAD changes
    println!("cargo:rerun-if-changed=../../.git/HEAD");
}
