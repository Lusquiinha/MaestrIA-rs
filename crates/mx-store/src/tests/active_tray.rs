use serde_json::{Value, json};

use super::Rig;

fn load(rig: &mut Rig, config: Value) {
    let state = rig.s.opts.state_dir.clone();
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("config.json"), config.to_string()).unwrap();
    rig.s.load();
}

fn tray(rig: &Rig) -> Vec<String> {
    rig.s.active_tray_tabs().iter().map(|t| t.id.clone()).collect()
}

#[test]
fn the_tray_starts_turned_off() {
    let rig = Rig::new();
    assert!(!rig.s.prefs().active_tray);
}

#[test]
fn a_prompt_puts_the_session_in_the_tray_when_it_is_on() {
    let mut rig = Rig::new();
    rig.panel("a");
    rig.panel("b");
    rig.s.set_active_tray(true);
    rig.hook("b", "UserPromptSubmit", json!({}));
    assert_eq!(tray(&rig), ["b"]);
}

#[test]
fn a_prompt_leaves_the_tray_alone_when_it_is_off() {
    let mut rig = Rig::new();
    rig.panel("a");
    rig.hook("a", "UserPromptSubmit", json!({}));
    assert!(tray(&rig).is_empty());
}

#[test]
fn a_shell_never_joins_the_tray_by_itself() {
    let mut rig = Rig::new();
    rig.shell("sh");
    rig.s.set_active_tray(true);
    rig.hook("sh", "UserPromptSubmit", json!({}));
    assert!(tray(&rig).is_empty());
}

#[test]
fn removing_from_the_tray_keeps_the_session_and_a_new_prompt_brings_it_back() {
    let mut rig = Rig::new();
    rig.panel("a");
    rig.s.set_active_tray(true);
    rig.hook("a", "UserPromptSubmit", json!({}));
    rig.s.set_in_active_tray("a", false);
    assert!(tray(&rig).is_empty());
    assert!(rig.s.tab("a").is_some());
    rig.hook("a", "Stop", json!({}));
    assert!(tray(&rig).is_empty());
    rig.hook("a", "UserPromptSubmit", json!({}));
    assert_eq!(tray(&rig), ["a"]);
}

#[test]
fn a_session_can_be_added_by_hand() {
    let mut rig = Rig::new();
    rig.panel("a");
    rig.s.set_active_tray(true);
    assert!(rig.s.can_join_active_tray("a"));
    rig.s.set_in_active_tray("a", true);
    assert_eq!(tray(&rig), ["a"]);
}

#[test]
fn hibernating_keeps_the_session_in_the_tray() {
    let mut rig = Rig::new();
    rig.panel("a");
    rig.tm("a").session_id = Some("sess-1".into());
    rig.s.set_active_tray(true);
    rig.turn("a");
    rig.s.hibernate("a");
    assert!(rig.t("a").hibernated);
    assert_eq!(tray(&rig), ["a"]);
}

#[test]
fn the_tray_and_its_sessions_are_saved() {
    let mut rig = Rig::new();
    rig.panel("a");
    rig.s.set_active_tray(true);
    rig.s.toggle_active_tray_collapsed();
    rig.s.set_in_active_tray("a", true);
    let saved = rig.written();
    assert_eq!(saved["activeTray"], true);
    assert_eq!(saved["activeTrayCollapsed"], true);
    assert_eq!(saved["layout"]["panes"][0]["activeTray"], true);
}

#[tokio::test]
async fn the_tray_and_its_sessions_come_back_on_load() {
    let mut rig = Rig::new();
    let cwd = rig.dir.path().to_str().unwrap().to_owned();
    let pane = |tray: bool| json!({"folderRoot": cwd, "kind": "shell", "cwd": cwd, "activeTray": tray});
    load(
        &mut rig,
        json!({
            "folders": [{"root": cwd, "name": "f", "collapsed": false}],
            "activeTray": true,
            "layout": {"panes": [pane(true), pane(false)]}
        }),
    );
    assert!(rig.s.prefs().active_tray);
    assert_eq!(rig.s.tabs().len(), 2);
    assert_eq!(rig.s.active_tray_tabs().len(), 1);
}
