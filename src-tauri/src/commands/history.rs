//! Git history rail handlers — ports of the GTK commit-click snapshot
//! browsing and rail visibility toggles.

use rendermd_core::history::{fetch_parent_sha, fetch_revision_text, iso_to_unix_secs};
use tauri::{AppHandle, Runtime};

use super::table::{refresh, toast};
use crate::state::{AppState, HistorySnapshot};

/// Rail commit click. Clicking the commit already being viewed returns to
/// the working copy; anything else fetches that revision (view-only — the
/// buffer of record is untouched) and diffs it against its parent for the
/// familiar yellow-bar + hover-diff view.
pub fn handle_commit_click<R: Runtime>(app: &AppHandle<R>, s: &mut AppState, sha: &str) {
    let sha = sha.trim();
    if sha.is_empty() {
        return;
    }

    // The rail's synthetic "Current" entry. No-op when already on the
    // working copy.
    if sha == "__working__" {
        if s.viewing_snapshot.take().is_some() {
            s.pending_changes = None;
            toast(app, "Returned to working copy");
            refresh(app, s);
        }
        return;
    }

    let already_viewing = s
        .viewing_snapshot
        .as_ref()
        .map(|snap| snap.sha == sha)
        .unwrap_or(false);
    if already_viewing {
        s.viewing_snapshot = None;
        s.pending_changes = None;
        toast(app, "Returned to working copy");
        refresh(app, s);
        return;
    }

    let Some(path) = s.file_path.clone() else {
        return;
    };
    let Some(text) = fetch_revision_text(&path, sha) else {
        toast(
            app,
            "Couldn't fetch that revision (file may have been renamed)",
        );
        return;
    };
    let parent_text =
        fetch_parent_sha(&path, sha).and_then(|psha| fetch_revision_text(&path, &psha));

    let commit = s
        .history
        .as_ref()
        .and_then(|cs| cs.iter().find(|c| c.sha == sha))
        .cloned();
    let commit_unix = commit
        .as_ref()
        .map(|c| iso_to_unix_secs(&c.iso_date))
        .unwrap_or(0);
    let toast_msg = match &commit {
        Some(c) => format!("Viewing {} — {}", c.short_sha, c.subject),
        None => format!("Viewing {}", &sha[..sha.len().min(7)]),
    };

    // render_preview re-derives pending_changes from the snapshot's parent
    // on every refresh, so just install the snapshot.
    s.pending_changes = None;
    s.viewing_snapshot = Some(HistorySnapshot {
        sha: sha.to_string(),
        text,
        parent_text,
        commit_unix_secs: commit_unix,
    });

    toast(app, &toast_msg);
    refresh(app, s);
}

pub fn handle_toggle_history<R: Runtime>(app: &AppHandle<R>, s: &mut AppState) {
    s.history_visible = !s.history_visible;
    if s.history.is_some() {
        toast(
            app,
            if s.history_visible {
                "History rail shown"
            } else {
                "History rail hidden"
            },
        );
    } else {
        toast(app, "This file isn't tracked in a git repo");
    }
    persist_flags(s);
    refresh(app, s);
}

pub fn handle_toggle_history_collapse<R: Runtime>(app: &AppHandle<R>, s: &mut AppState) {
    s.history_collapsed = !s.history_collapsed;
    persist_flags(s);
    refresh(app, s);
}

fn persist_flags(s: &AppState) {
    crate::settings::save(&crate::settings::Settings {
        history_visible: s.history_visible,
        history_collapsed: s.history_collapsed,
    });
}
