use mx_core::model::ClaudeStatus;
use mx_core::persist::Layout;
use serde_json::{Value, json};

use super::Rig;

fn pane(rig: &mut Rig, kind: &str, tab_id: &str) -> serde_json::Map<String, Value> {
    let root = rig.mkdir("repo");
    rig.folder(&root, "repo");
    json!({"folderRoot": root, "kind": kind, "cwd": root, "sessionId": "sess-1", "tabId": tab_id})
        .as_object()
        .cloned()
        .unwrap()
}

fn restore(rig: &mut Rig, panes: Vec<serde_json::Map<String, Value>>) {
    rig.s.restore_layout(&Layout { panes, ..Layout::default() });
}

#[test]
fn a_claude_still_running_in_the_daemon_is_adopted_under_its_old_id() {
    let mut rig = Rig::new();
    rig.terms.still_running("tabold_3");
    let p = pane(&mut rig, "claude", "tabold_3");
    restore(&mut rig, vec![p]);
    let tab = rig.t("tabold_3");
    assert_eq!(tab.session_id.as_deref(), Some("sess-1"));
    assert_eq!(tab.status(), ClaudeStatus::Ready);
    assert!(!tab.exited);
    assert!(rig.terms.log("tabold_3").commands.is_empty());
}

#[test]
fn a_shell_still_running_is_adopted_without_a_new_shell() {
    let mut rig = Rig::new();
    rig.terms.still_running("tabold_1");
    let p = pane(&mut rig, "shell", "tabold_1");
    restore(&mut rig, vec![p]);
    assert!(rig.terms.log("tabold_1").shells.is_empty());
}

#[test]
fn a_session_that_is_gone_is_launched_again_under_a_new_id() {
    let mut rig = Rig::new();
    let p = pane(&mut rig, "claude", "tabold_3");
    restore(&mut rig, vec![p]);
    assert!(rig.s.tab("tabold_3").is_none());
    let id = rig.s.tabs().first().unwrap().id.clone();
    assert!(rig.terms.log(&id).commands[0].contains("--resume") && rig.terms.log(&id).commands[0].contains("sess-1"));
}

#[tokio::test]
async fn restarting_for_an_update_keeps_the_sessions() {
    let mut rig = Rig::new();
    let id = rig.shell("s");
    rig.s.relaunch_after_quit();
    rig.s.shutdown().await;
    let log = rig.terms.log(&id);
    assert_eq!((log.kills, log.kept), (0, true));
}

#[tokio::test]
async fn quitting_still_ends_the_sessions() {
    let mut rig = Rig::new();
    let id = rig.shell("s");
    rig.s.shutdown().await;
    let log = rig.terms.log(&id);
    assert_eq!((log.kills, log.kept), (1, false));
}

#[test]
fn the_restored_layout_is_saved_so_a_crash_right_after_keeps_the_ids() {
    let mut rig = Rig::new();
    let p = pane(&mut rig, "shell", "tabold_1");
    restore(&mut rig, vec![p]);
    let id = rig.s.tabs().first().unwrap().id.clone();
    let saved = rig.written();
    assert_eq!(saved["layout"]["panes"][0]["tabId"], json!(id));
}

#[test]
fn a_claude_that_finished_while_the_app_was_closed_comes_back_ended() {
    let mut rig = Rig::new();
    rig.terms.finished_while_away("tabold_3", 0);
    let p = pane(&mut rig, "claude", "tabold_3");
    restore(&mut rig, vec![p]);
    let tab = rig.t("tabold_3");
    assert!(tab.exited);
    assert_eq!(tab.exit_code, Some(0));
    assert_eq!(tab.status(), ClaudeStatus::Ended);
    assert!(rig.terms.log("tabold_3").commands.is_empty());
}

#[test]
fn ending_sessions_on_quit_is_a_saved_preference() {
    let mut rig = Rig::new();
    assert!(!rig.s.prefs().end_sessions_on_quit);
    rig.s.set_end_sessions_on_quit(true);
    let saved = rig.written();
    assert_eq!(saved["endSessionsOnQuit"], json!(true));
}

#[test]
fn a_claude_hibernated_while_the_app_was_closed_comes_back_asleep_and_wakes_with_resume() {
    let mut rig = Rig::new();
    rig.terms.hibernated_while_away("tabold_3");
    let p = pane(&mut rig, "claude", "tabold_3");
    restore(&mut rig, vec![p]);
    assert!(rig.t("tabold_3").hibernated);
    assert_eq!(rig.terms.log("tabold_3").remarks, [crate::HIBERNATED]);
    rig.s.wake("tabold_3");
    let commands = rig.terms.log("tabold_3").commands;
    assert!(commands.len() == 1 && commands[0].contains("sess-1"));
}

#[test]
fn quitting_hands_the_daemon_only_resumable_claudes_and_the_hibernation_time() {
    let mut rig = Rig::new();
    rig.s1();
    rig.add_tab("c", "/repo", mx_core::model::TabKind::Claude, "/repo", None);
    rig.tm("c").session_id = Some("sess-9".into());
    rig.add_tab("n", "/repo", mx_core::model::TabKind::Claude, "/repo", None);
    rig.shell("s");
    rig.s.set_hibernate_minutes(30);
    let (ids, after) = rig.s.hibernatable_away().unwrap();
    assert_eq!((ids, after.as_secs()), (vec!["c".to_owned()], 1800));
    rig.s.set_hibernate_minutes(0);
    assert!(rig.s.hibernatable_away().is_none());
}
