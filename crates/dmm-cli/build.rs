use std::process::Command;

fn main() {
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

    // The icon Explorer and shortcuts show for dmm-cli.exe, and the version in
    // Properties, as for dmm-gui (crates/dmm-gui/build.rs).
    // scripts/make-ico.ps1 builds icon-cli.ico from icon-cli.svg.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=../../assets/icon-cli.ico");
        let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/icon-cli.ico");
        // Not worth failing the build over: a toolchain without a resource
        // compiler (windres, llvm-rc) still builds, iconless, and says so.
        if let Err(e) = winresource::WindowsResource::new()
            .set_icon(icon)
            .compile_for("dmm-cli")
        {
            println!("cargo:warning=dmm-cli.exe built without its icon: {e}");
        }
    }
}
