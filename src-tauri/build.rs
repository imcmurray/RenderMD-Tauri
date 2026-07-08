fn main() {
    // Embed the short git SHA for the About dialog (port of the GTK repo's
    // build.rs). Resolution order: GIT_SHA env (CI/packaging builds need no
    // .git), `git rev-parse`, then "unknown" (tarball builds).
    let sha = std::env::var("GIT_SHA").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| {
        std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".into())
    });
    println!("cargo:rustc-env=GIT_SHA={sha}");
    println!("cargo:rerun-if-env-changed=GIT_SHA");

    tauri_build::build()
}
