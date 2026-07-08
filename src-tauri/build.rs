fn main() {
    // Embed the short git SHA for the About dialog (port of the GTK repo's
    // build.rs). Resolution order: GIT_SHA env (CI/packaging builds need no
    // .git), `git rev-parse`, then "unknown" (tarball builds).
    let sha = std::env::var("GIT_SHA")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
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

    // Recent-changes list for the welcome page, embedded at build time so
    // the installed app needs neither a .git dir nor network. Multi-line,
    // so it travels via OUT_DIR file + include_str! rather than rustc-env.
    let recent = std::process::Command::new("git")
        .args([
            "log",
            "-n",
            "8",
            "--pretty=format:- %s (`%h`, %ad)",
            "--date=short",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "- See the repository for the full changelog.".into());
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    std::fs::write(
        std::path::Path::new(&out_dir).join("recent_changes.md"),
        recent,
    )
    .expect("write recent_changes.md");

    tauri_build::build()
}
