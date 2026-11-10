use std::process::Command;

fn main() -> std::io::Result<()> {
    // Embed the git commit hash only in release builds. Debug builds use a
    // constant placeholder so the binary's compile-time identity stays fixed
    // across commits — otherwise each new commit's hash mints a fresh
    // *-<hash> artifact in target/debug/deps that Cargo never garbage-collects
    // (see docs/development.md). The release pipeline always builds --release,
    // so distributed binaries still carry the real hash.
    let hash = if std::env::var("PROFILE").as_deref() == Ok("release") {
        let output = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output();
        match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            _ => "unknown".to_string(),
        }
    } else {
        "dev".to_string()
    };

    println!("cargo:rustc-env=GIT_HASH={hash}");
    // Re-run when HEAD moves (affects release stamping).
    println!("cargo:rerun-if-changed=../../.git/HEAD");

    // Explorer, the Start menu and shortcuts read an exe's icon from its
    // embedded resources; the runtime window icon (main.rs) never reaches
    // them. The resource also carries the version shown in Properties.
    // icon.ico is icon-256.png at 16 to 256 px: rebuild it with the PNG.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=../../assets/icon.ico");
        winresource::WindowsResource::new()
            .set_icon("../../assets/icon.ico")
            .compile()?;
    }
    Ok(())
}
