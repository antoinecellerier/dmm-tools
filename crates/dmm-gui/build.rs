use std::process::Command;

fn main() {
    // Embed the git commit hash only in release builds. Debug builds use a
    // constant placeholder so the binary's compile-time identity stays fixed
    // across commits — otherwise each new commit's hash mints a fresh
    // *-<hash> artifact in target/debug/deps that Cargo never garbage-collects
    // (see docs/development.md). The release pipeline always builds --release,
    // so distributed binaries still carry the real hash.
    let release = std::env::var("PROFILE").as_deref() == Ok("release");
    let hash = if release {
        git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".to_string())
    } else {
        "dev".to_string()
    };
    println!("cargo:rustc-env=GIT_HASH={hash}");

    // Naming any file replaces Cargo's default of rerunning on every change
    // in the package. A debug build stamps no hash, so watches nothing of git's.
    println!("cargo:rerun-if-changed=build.rs");
    if release {
        watch_head();
    }

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

/// Reruns this script when HEAD moves. `HEAD` alone only names the branch and
/// stays the same across commits, so the moves are read from its reflog
/// (`logs/HEAD`), which every commit, checkout, reset and pull appends to.
/// Git resolves the paths: in a worktree they are not under `../../.git`.
fn watch_head() {
    let mut files = vec!["HEAD".to_string(), "logs/HEAD".to_string()];
    files.extend(git(&["symbolic-ref", "-q", "HEAD"]));
    for file in files {
        let Some(path) = git(&["rev-parse", "--path-format=absolute", "--git-path", &file]) else {
            continue;
        };
        // Cargo reruns on every build for a file that does not exist, such as
        // a branch ref that only lives in packed-refs.
        if std::path::Path::new(&path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    output.status.success().then_some(text)
}
