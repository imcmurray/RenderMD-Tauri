//! Git history for the open file: `git log`/`git show` plumbing plus the
//! HTML for the preview's commit-timeline rail. Ported from the GTK app's
//! `main.rs` with three swaps: glib date parsing → chrono, WebKit
//! `messageHandlers` postMessage → the shell-agnostic `window.__rmdPost`
//! shim, and a `git_command()` helper that suppresses console windows on
//! Windows.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

use chrono::{Local, TimeZone};

use crate::render::html_escape;

// One commit affecting the open file, returned by `git log` and used to
// populate the history rail in the preview. `sha` is the full hash;
// `short_sha` is what we display.
#[derive(Clone, Debug)]
pub struct Commit {
    pub sha: String,
    pub short_sha: String,
    pub iso_date: String,
    pub subject: String,
    pub additions: u32,
    pub deletions: u32,
}

// All git invocations go through here so the Windows console suppression
// is applied uniformly. 0x08000000 is CREATE_NO_WINDOW: without it every
// `git` call flashes a conhost window when the app runs outside a console.
fn git_command() -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}

// Run `git log --follow` for the file and return the commit list, newest
// first. Returns None if git isn't installed, the file isn't in a repo,
// the file isn't tracked, or git errored. Capped at 100 commits to keep
// the rail manageable on long-lived files; a "show more" affordance is
// future work.
pub fn fetch_git_history(file_path: &Path) -> Option<Vec<Commit>> {
    let parent = file_path.parent()?;
    let file_name = file_path.file_name()?.to_str()?;

    // Cheap check: are we inside a working tree at all?
    let in_repo = git_command()
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(parent)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some();
    if !in_repo {
        return None;
    }

    // `--numstat` appends per-file "added<TAB>deleted<TAB>path" lines after
    // each commit. We prefix the pretty header with \x01 (SOH) so the two
    // line kinds are unambiguous to parse — content never contains it.
    let output = git_command()
        .args([
            "log",
            "--follow",
            "-n",
            "100",
            "--numstat",
            "--pretty=format:\x01%H%x09%h%x09%cI%x09%s",
            "--",
            file_name,
        ])
        .current_dir(parent)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits: Vec<Commit> = Vec::new();
    for line in stdout.lines() {
        if let Some(header) = line.strip_prefix('\x01') {
            let mut parts = header.splitn(4, '\t');
            let (Some(sha), Some(short_sha), Some(iso_date), Some(subject)) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            commits.push(Commit {
                sha: sha.to_string(),
                short_sha: short_sha.to_string(),
                iso_date: iso_date.to_string(),
                subject: subject.to_string(),
                additions: 0,
                deletions: 0,
            });
        } else if !line.trim().is_empty() {
            // numstat row for the current commit: "added\tdeleted\tpath".
            // Binary files report "-" for both; parse failures count as 0.
            if let Some(c) = commits.last_mut() {
                let mut p = line.splitn(3, '\t');
                c.additions += p.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                c.deletions += p.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            }
        }
    }
    if commits.is_empty() {
        None
    } else {
        Some(commits)
    }
}

// Format a strict-ISO commit date ("2026-06-27T23:49:12-06:00") as a compact
// "YYYY-MM-DD HH:MM" for the history rail. Falls back to the raw date portion
// if the time can't be extracted.
pub fn format_commit_datetime(iso: &str) -> String {
    let mut sp = iso.splitn(2, 'T');
    let date = sp.next().unwrap_or(iso);
    let time = sp.next().unwrap_or("");
    let hm: String = time.chars().take(5).collect();
    if hm.len() == 5 {
        format!("{date} {hm}")
    } else {
        date.to_string()
    }
}

// Compact "time ago" label for the history rail ("just now", "5m ago",
// "3h ago", "2d ago", "4w ago", "6mo ago", "2y ago"). Returns "" when the
// timestamp can't be parsed (0) so the caller can fall back to the date.
pub fn format_relative_time(then_secs: i64, now_secs: i64) -> String {
    if then_secs <= 0 {
        return String::new();
    }
    let d = now_secs - then_secs;
    if d < 60 {
        return "just now".to_string();
    }
    let (n, unit) = if d < 3_600 {
        (d / 60, "m")
    } else if d < 86_400 {
        (d / 3_600, "h")
    } else if d < 604_800 {
        (d / 86_400, "d")
    } else if d < 2_592_000 {
        (d / 604_800, "w")
    } else if d < 31_536_000 {
        (d / 2_592_000, "mo")
    } else {
        (d / 31_536_000, "y")
    };
    format!("{n}{unit} ago")
}

// Resolve the working tree's toplevel and the file's path relative to
// it. Needed for `git show <sha>:<relpath>`.
pub fn repo_relative(file_path: &Path) -> Option<(PathBuf, String)> {
    let parent = file_path.parent()?;
    let output = git_command()
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(parent)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let toplevel = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let toplevel_path = PathBuf::from(toplevel);
    let rel = file_path.strip_prefix(&toplevel_path).ok()?;
    Some((toplevel_path, rel.to_string_lossy().to_string()))
}

// Read the file's content at a given commit. Returns None for renamed
// files (would need `git log --follow --name-only` to track) or other
// `git show` failures.
pub fn fetch_revision_text(file_path: &Path, sha: &str) -> Option<String> {
    let (toplevel, rel) = repo_relative(file_path)?;
    let arg = format!("{sha}:{rel}");
    let output = git_command()
        .args(["show", &arg])
        .current_dir(&toplevel)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

// Get the parent SHA of a given commit. Returns None for the root
// commit (no parent) or on any git error.
pub fn fetch_parent_sha(file_path: &Path, sha: &str) -> Option<String> {
    let parent = file_path.parent()?;
    let parent_arg = format!("{sha}^");
    let output = git_command()
        .args(["rev-parse", "--quiet", &parent_arg])
        .current_dir(parent)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn iso_to_unix_secs(iso: &str) -> i64 {
    // git's %cI output is RFC 3339 ("2026-06-27T23:49:12-06:00").
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|d| d.timestamp())
        .unwrap_or(0)
}

pub fn format_mtime(path: &Path) -> String {
    let modified = match fs::metadata(path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return String::new(),
    };
    let secs = match modified.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(_) => return String::new(),
    };
    match Local.timestamp_opt(secs, 0).single() {
        Some(dt) => format!("Updated {}", dt.format("%Y-%m-%d %H:%M")),
        None => String::new(),
    }
}

// Build the HTML for the left-side commit timeline. Returns the rail
// markup plus a small <script> that wires click messages back to Rust.
// Empty string when there's no history. When `visible` is false but
// commits exist, returns just an unobtrusive hint dot so the user
// knows the option is there.
/// Derive the repo's browseable web URL from its `origin` remote, for
/// per-commit deep links (`<url>/commit/<sha>` works across GitHub, GitLab,
/// and Gitea). Returns None when the file isn't in a repo or the remote
/// isn't URL-shaped.
pub fn remote_web_url(file_path: &Path) -> Option<String> {
    let dir = file_path.parent()?;
    let out = git_command()
        .args(["-C"])
        .arg(dir)
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let raw = raw.strip_suffix(".git").unwrap_or(&raw).to_string();
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return Some(raw);
    }
    // git@host:owner/repo → https://host/owner/repo
    if let Some(rest) = raw.strip_prefix("git@") {
        if let Some((host, path)) = rest.split_once(':') {
            return Some(format!("https://{host}/{path}"));
        }
    }
    // ssh://git@host/owner/repo → https://host/owner/repo
    if let Some(rest) = raw.strip_prefix("ssh://") {
        let rest = rest.strip_prefix("git@").unwrap_or(rest);
        return Some(format!("https://{rest}"));
    }
    None
}

pub fn build_history_rail_html(
    commits: &[Commit],
    viewing_sha: Option<&str>,
    visible: bool,
    collapsed: bool,
    remote_url: Option<&str>,
) -> String {
    if commits.is_empty() {
        return String::new();
    }
    if !visible {
        // Click handler so the hint dot itself toggles history back on.
        return r##"<div class="rmd-history-hint" title="Git history available — click to show"></div>
<script>
(function() {
  if (!window.__rmdPost) return;
  var hint = document.querySelector(".rmd-history-hint");
  if (hint) hint.addEventListener("click", function() {
    window.__rmdPost("toggleHistory", "");
  });
})();
</script>"##
            .to_string();
    }

    let rail_cls = if collapsed {
        "rmd-history-rail rmd-collapsed"
    } else {
        "rmd-history-rail"
    };
    let mut html =
        format!(r#"<div class="{rail_cls}" role="navigation" aria-label="Commit history">"#,);
    // Collapse/expand toggle: chevron points the way it will move the panel.
    let (chevron, collapse_title) = if collapsed {
        ("»", "Expand history labels")
    } else {
        ("«", "Collapse to dots")
    };
    html.push_str(&format!(
        r#"<button type="button" class="rmd-history-collapse" title="{title}" aria-label="{title}">{chevron}</button>"#,
        title = html_escape(collapse_title),
        chevron = chevron,
    ));
    html.push_str(r#"<div class="rmd-history-track"></div>"#);

    // Synthetic "Current" entry: always first, active whenever no snapshot
    // is being viewed, and the discoverable way back to the working copy
    // (clicking the active commit again still works too).
    {
        let cls = if viewing_sha.is_none() {
            "rmd-history-circle rmd-history-current rmd-history-active"
        } else {
            "rmd-history-circle rmd-history-current"
        };
        html.push_str(&format!(
            r#"<div class="rmd-history-item"><button type="button" class="{cls}" data-sha="__working__" title="Working copy — the latest saved version"><span class="rmd-history-dot"></span><span class="rmd-history-info"><span class="rmd-history-when">Current</span></span></button></div>"#,
        ));
    }

    let now_secs = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    for c in commits {
        let active = viewing_sha.map(|v| v == c.sha).unwrap_or(false);
        let when_abs = format_commit_datetime(&c.iso_date);
        // Visible label is relative ("2d ago"); the absolute date stays in the
        // hover tooltip. Fall back to the absolute date if it won't parse.
        let rel = format_relative_time(iso_to_unix_secs(&c.iso_date), now_secs);
        let when_label = if rel.is_empty() {
            when_abs.clone()
        } else {
            rel
        };
        let tooltip = format!(
            "{} — {}\n+{} −{}\n{}",
            c.short_sha, when_abs, c.additions, c.deletions, c.subject
        );
        let cls = if active {
            "rmd-history-circle rmd-history-active"
        } else {
            "rmd-history-circle"
        };
        // Optional deep link to the commit on the remote host — a sibling
        // button (nesting buttons is invalid HTML), shown on hover.
        let link = match remote_url {
            Some(url) => format!(
                r#"<button type="button" class="rmd-history-link" data-url="{url}/commit/{sha}" title="Open commit on remote" aria-label="Open commit on remote">↗</button>"#,
                url = html_escape(url),
                sha = html_escape(&c.sha),
            ),
            None => String::new(),
        };
        html.push_str(&format!(
            r#"<div class="rmd-history-item"><button type="button" class="{cls}" data-sha="{sha}" title="{title}"><span class="rmd-history-dot"></span><span class="rmd-history-info"><span class="rmd-history-when">{when}</span><span class="rmd-history-stat"><span class="rmd-history-add">+{add}</span><span class="rmd-history-del">−{del}</span></span></span></button>{link}</div>"#,
            cls = cls,
            sha = html_escape(&c.sha),
            title = html_escape(&tooltip),
            when = html_escape(&when_label),
            add = c.additions,
            del = c.deletions,
            link = link,
        ));
    }
    html.push_str("</div>");
    // Click handler — Phase 2 will hook this up to render the chosen
    // revision. For now, posts the SHA to the `commitClick` channel
    // so we can verify wiring end-to-end.
    html.push_str(
        r#"<script>
(function() {
  if (!window.__rmdPost) return;
  document.querySelectorAll(".rmd-history-circle").forEach(function(c) {
    c.addEventListener("click", function() {
      window.__rmdPost("commitClick", c.getAttribute("data-sha") || "");
    });
  });
  document.querySelectorAll(".rmd-history-link").forEach(function(l) {
    l.addEventListener("click", function(e) {
      e.stopPropagation();
      window.__rmdPost("openExternal", l.getAttribute("data-url") || "");
    });
  });
  var tog = document.querySelector(".rmd-history-collapse");
  if (tog) tog.addEventListener("click", function() {
    window.__rmdPost("toggleHistoryCollapse", "");
  });
})();
</script>"#,
    );
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_to_unix_secs_parses_git_ci_output() {
        // git %cI is RFC 3339 with a numeric offset.
        assert_eq!(iso_to_unix_secs("1970-01-01T00:00:00+00:00"), 0);
        // 2026-06-27T23:49:12-06:00 == 2026-06-28T05:49:12Z == 1782625752.
        assert_eq!(iso_to_unix_secs("2026-06-27T23:49:12-06:00"), 1782625752);
        assert_eq!(iso_to_unix_secs("not a date"), 0);
    }

    #[test]
    fn commit_datetime_compacts_to_minutes() {
        assert_eq!(
            format_commit_datetime("2026-06-27T23:49:12-06:00"),
            "2026-06-27 23:49"
        );
        // No time portion → date only.
        assert_eq!(format_commit_datetime("2026-06-27"), "2026-06-27");
    }

    #[test]
    fn relative_time_buckets() {
        let now = 1_000_000_000i64;
        assert_eq!(format_relative_time(0, now), "");
        assert_eq!(format_relative_time(now - 30, now), "just now");
        assert_eq!(format_relative_time(now - 300, now), "5m ago");
        assert_eq!(format_relative_time(now - 3 * 3_600, now), "3h ago");
        assert_eq!(format_relative_time(now - 2 * 86_400, now), "2d ago");
        assert_eq!(format_relative_time(now - 4 * 604_800, now), "4w ago");
        assert_eq!(format_relative_time(now - 6 * 2_592_000, now), "6mo ago");
        assert_eq!(format_relative_time(now - 2 * 31_536_000, now), "2y ago");
    }

    #[test]
    fn rail_html_uses_rmdpost_shim() {
        let commits = vec![Commit {
            sha: "abc123".into(),
            short_sha: "abc".into(),
            iso_date: "2026-06-27T23:49:12-06:00".into(),
            subject: "subject".into(),
            additions: 1,
            deletions: 2,
        }];
        let html = build_history_rail_html(&commits, None, true, false, None);
        assert!(html.contains(r#"window.__rmdPost("commitClick""#));
        assert!(html.contains(r#"window.__rmdPost("toggleHistoryCollapse""#));
        assert!(!html.contains("webkit.messageHandlers"));

        let hidden = build_history_rail_html(&commits, None, false, false, None);
        assert!(hidden.contains(r#"window.__rmdPost("toggleHistory""#));
        assert!(!hidden.contains("webkit.messageHandlers"));
    }

    #[test]
    fn rail_html_empty_without_commits() {
        assert_eq!(build_history_rail_html(&[], None, true, false, None), "");
    }

    #[test]
    fn rail_current_entry_first_and_active_on_working_copy() {
        let commits = vec![Commit {
            sha: "abc123".into(),
            short_sha: "abc".into(),
            iso_date: "2026-06-27T23:49:12-06:00".into(),
            subject: "subject".into(),
            additions: 1,
            deletions: 2,
        }];
        let html = build_history_rail_html(&commits, None, true, false, None);
        let current = html.find("__working__").expect("Current entry present");
        let commit = html.find("abc123").expect("commit present");
        assert!(current < commit, "Current entry must come first");
        assert!(html.contains("rmd-history-current rmd-history-active"));

        // Viewing a snapshot: Current no longer active, the commit is.
        let viewing = build_history_rail_html(&commits, Some("abc123"), true, false, None);
        assert!(!viewing.contains("rmd-history-current rmd-history-active"));
    }

    #[test]
    fn rail_commit_links_only_with_remote() {
        let commits = vec![Commit {
            sha: "abc123".into(),
            short_sha: "abc".into(),
            iso_date: "2026-06-27T23:49:12-06:00".into(),
            subject: "subject".into(),
            additions: 1,
            deletions: 2,
        }];
        // (The wiring JS always mentions the selector; assert on the button
        // markup itself.)
        let without = build_history_rail_html(&commits, None, true, false, None);
        assert!(!without.contains(r#"class="rmd-history-link""#));

        let with = build_history_rail_html(
            &commits,
            None,
            true,
            false,
            Some("https://github.com/u/repo"),
        );
        assert!(with.contains(r#"data-url="https://github.com/u/repo/commit/abc123""#));
        assert!(with.contains(r#"window.__rmdPost("openExternal""#));
        // The synthetic Current entry never gets a commit link.
        assert!(!with.contains("commit/__working__"));
    }
}
