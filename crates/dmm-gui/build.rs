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

    // Explorer, the Start menu and shortcuts read an exe's icon from its
    // embedded resources; the window icon main.rs sets at runtime never
    // reaches them. The resource also carries the version shown in Properties.
    // scripts/make-ico.ps1 builds icon.ico from icon.svg.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=../../assets/icon.ico");
        let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/icon.ico");
        // Not worth failing the build over: a toolchain without a resource
        // compiler (windres, llvm-rc) still builds, iconless, and says so.
        if let Err(e) = winresource::WindowsResource::new()
            .set_icon(icon)
            .compile_for("dmm-gui")
        {
            println!("cargo:warning=dmm-gui.exe built without its icon: {e}");
        }
    }
}
