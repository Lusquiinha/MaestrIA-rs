use gpui::{
    Bounds, Entity, Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px,
};
use mx_core::model::sidebar::RowKey;
use serde_json::json;
use tempfile::TempDir;

use crate::app::host::StoreHost;
use crate::app::theme::Mx;
use crate::sidebar::Sidebar;
use crate::test_store::rig;

struct Rig {
    host: Entity<StoreHost>,
    ws: String,
    _dir: TempDir,
}

impl Rig {
    fn rows(&self, cx: &mut VisualTestContext) -> Vec<String> {
        self.host.read_with(cx, |h, _| h.store.sidebar_rows().iter().map(RowKey::key).collect())
    }

    fn members(&self, cx: &mut VisualTestContext) -> Vec<String> {
        self.host.read_with(cx, |h, _| h.store.folders_of(&self.ws).iter().map(|f| f.root.clone()).collect())
    }
}

/// Root folders `/um`, `/dois`, `/solta` and a workspace holding `/api`.
fn mount(cx: &mut TestAppContext) -> (Rig, &mut VisualTestContext) {
    let r = rig();
    let mut store = r.s;
    let state = r.dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let folder = |root: &str| json!({"root": root, "name": root.trim_start_matches('/'), "collapsed": true});
    let config = json!({"folders": [folder("/um"), folder("/dois"), folder("/solta"), folder("/api")]});
    std::fs::write(state.join("config.json"), config.to_string()).unwrap();
    store.load();
    let ws = store.create_workspace("cefis", &["/api"], None).unwrap();
    let rt = r.rt;
    let host = cx.update(|cx| {
        Mx::install(mx_theme::default_theme(), cx);
        let host = StoreHost::for_test(store, rt, cx);
        crate::app::ui::install(host.clone(), cx);
        host
    });
    let h = host.clone();
    let (_, vcx) = cx.add_window_view(move |window, cx| Sidebar::new(h, window, cx));
    vcx.run_until_parked();
    (Rig { host, ws, _dir: r.dir }, vcx)
}

fn bounds(cx: &mut VisualTestContext, selector: String) -> Bounds<Pixels> {
    let key: &'static str = Box::leak(selector.clone().into_boxed_str());
    cx.debug_bounds(key).unwrap_or_else(|| panic!("{selector} drawn"))
}

fn folder(root: &str, within: Option<&str>) -> String {
    format!("row:folder:{root}@{}", within.unwrap_or(""))
}

fn drag_to(cx: &mut VisualTestContext, from: String, to: Point<Pixels>) {
    let from = bounds(cx, from).center();
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
}

#[gpui::test]
fn a_folder_dragged_onto_another_takes_its_slot(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx);
    assert_eq!(rig.rows(cx)[..3], ["folder:/um", "folder:/dois", "folder:/solta"]);
    let to = bounds(cx, folder("/um", None)).center();
    drag_to(cx, folder("/solta", None), to);
    assert_eq!(rig.rows(cx)[..3], ["folder:/solta", "folder:/um", "folder:/dois"]);
}

#[gpui::test]
fn a_folder_dropped_on_the_middle_of_a_workspace_header_joins_it(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx);
    let to = bounds(cx, format!("row:workspace:{}@", rig.ws)).center();
    drag_to(cx, folder("/solta", None), to);
    assert_eq!(rig.members(cx), ["/api", "/solta"]);
    assert!(!rig.rows(cx).contains(&"folder:/solta".to_owned()));
}

#[gpui::test]
fn a_folder_dropped_on_the_edge_of_a_workspace_header_only_reorders(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx);
    let header = bounds(cx, format!("row:workspace:{}@", rig.ws));
    drag_to(cx, folder("/um", None), point(header.center().x, header.top() + px(2.)));
    assert_eq!(rig.members(cx), ["/api"]);
    assert_eq!(rig.rows(cx).last().map(String::as_str), Some("folder:/um"));
}

#[gpui::test]
fn a_folder_dragged_out_of_a_workspace_lands_at_the_root(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx);
    let to = bounds(cx, folder("/um", None)).center();
    drag_to(cx, folder("/api", Some(&rig.ws)), to);
    assert!(rig.members(cx).is_empty());
    assert_eq!(rig.rows(cx)[0], "folder:/api");
}

#[gpui::test]
fn a_drag_does_not_toggle_the_folder_it_started_on(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx);
    let to = bounds(cx, folder("/um", None)).center();
    drag_to(cx, folder("/solta", None), to);
    let collapsed = rig.host.read_with(cx, |h, _| h.store.is_folder_collapsed("/solta", None));
    assert!(collapsed);
}
