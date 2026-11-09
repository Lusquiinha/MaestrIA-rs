use mx_plugins::Plugins;
use mx_plugins::rpc::{OpenShell, ViewOpen};
use serde_json::{Value, json};

use super::Rig;

fn write_plugin(root: &str, id: &str, manifest: Value) {
    let dir = std::path::Path::new(root).join(id);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(mx_plugins::MANIFEST_NAME), manifest.to_string()).unwrap();
}

fn tool(id: &str, commands: Value) -> Value {
    json!({"id": id, "version": "1", "main": ["true"], "contributes": {"commands": commands}})
}

fn button(id: &str) -> Value {
    json!({"id": id, "title": id, "icon": "cpu", "sidebar": true})
}

fn desk(plugins: &[(&str, Value)], disabled: &[&str]) -> Rig {
    let mut rig = Rig::new();
    let root = rig.mkdir("plugins");
    let data = rig.mkdir("plugin-data");
    for (id, m) in plugins {
        write_plugin(&root, id, m.clone());
    }
    rig.s.prefs.plugins = json!({"disabled": disabled}).as_object().unwrap().clone();
    rig.s.attach_plugins(Plugins::with_dirs(root.into(), data.into()));
    rig.s1();
    rig.panel("s");
    rig.s.look("s");
    rig
}

fn footer(rig: &Rig) -> Vec<String> {
    rig.s.footer_commands().iter().map(|c| c.full_id()).collect()
}

#[test]
fn only_sidebar_commands_become_footer_buttons_in_plugin_order() {
    let rig = desk(
        &[
            ("b", tool("b", json!([button("um"), {"id": "menu", "title": "só no menu"}]))),
            ("a", tool("a", json!([button("dois")]))),
        ],
        &[],
    );
    assert_eq!(footer(&rig), ["a/dois", "b/um"]);
}

#[test]
fn the_footer_holds_three_buttons_at_most() {
    let rig = desk(
        &[
            ("a", tool("a", json!([button("1"), button("2")]))),
            ("b", tool("b", json!([button("3"), button("4")]))),
        ],
        &[],
    );
    assert_eq!(footer(&rig), ["a/1", "a/2", "b/3"]);
}

#[test]
fn a_disabled_plugin_has_no_footer_button() {
    let rig = desk(
        &[("a", tool("a", json!([button("x")]))), ("b", tool("b", json!([button("y")])))],
        &["a"],
    );
    assert_eq!(footer(&rig), ["b/y"]);
}

#[test]
fn rail_plugins_do_not_take_footer_slots_from_the_others() {
    let mut rail = tool("a", json!([button("1"), button("2"), button("3")]));
    rail["icon"] = json!("terminal");
    let rig = desk(&[("a", rail), ("b", tool("b", json!([button("x")])))], &[]);
    assert_eq!(rig.s.rail_plugins(), ["a"]);
    assert_eq!(footer(&rig), ["b/x"]);
}

#[test]
fn an_embedded_terminal_in_a_modal_keeps_the_button_off_the_rail() {
    let mut rig = desk(&[("btop", tool("btop", json!([button("abrir")])))], &[]);
    let shell = OpenShell { cwd: None, command: Some("btop".into()), label: None, owned: true, tag: None, embedded: true };
    let term = rig.s.plugin_open_shell("btop", shell);
    let p = rig.s.plugins().unwrap().get("btop").unwrap();
    let blocks = vec![json!({"type": "terminal", "tabId": term}).as_object().unwrap().clone()];
    rig.s.view_open(&p, ViewOpen { view_id: "btop".into(), title: "btop".into(), blocks, modal: true, ..ViewOpen::default() });
    assert!(rig.s.modal().is_some());
    assert!(rig.s.rail_plugins().is_empty());
    assert_eq!(rig.s.open_windows("btop"), 0);
    assert_eq!(footer(&rig), ["btop/abrir"]);
}

#[test]
fn command_busy_follows_the_registry() {
    let rig = desk(&[("a", tool("a", json!([button("x")])))], &[]);
    assert!(!rig.s.command_busy("a/x"));
    rig.s.plugins().unwrap().set_busy("a/x", true);
    assert!(rig.s.command_busy("a/x"));
}
