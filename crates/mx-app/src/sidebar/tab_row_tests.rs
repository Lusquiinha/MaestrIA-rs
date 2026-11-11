use gpui::{
    AppContext, Bounds, Context, Entity, IntoElement, Modifiers, MouseButton, ParentElement, Point, Pixels, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, point, px,
};
use mx_core::model::{FeatureOrHotfixKind, Folder};
use serde_json::json;
use tempfile::TempDir;

use crate::app::drag::GHOST_ANCHOR;
use crate::app::host::{StoreChanged, StoreHost};
use crate::app::theme::Mx;
use crate::panes::PaneArea;
use crate::sidebar::Sidebar;
use crate::test_store::rig;

struct Cockpit {
    sidebar: Entity<Sidebar>,
    panes: Entity<PaneArea>,
}

impl Cockpit {
    fn new(host: Entity<StoreHost>, window: &mut Window, cx: &mut Context<Self>) -> Cockpit {
        cx.subscribe(&host, |this: &mut Cockpit, _, _: &StoreChanged, cx| {
            this.sidebar.update(cx, |_, cx| cx.notify());
            this.panes.update(cx, |_, cx| cx.notify());
        })
        .detach();
        let sidebar = cx.new(|cx| Sidebar::new(host.clone(), window, cx));
        let panes = cx.new(|_| PaneArea::new(host));
        Cockpit { sidebar, panes }
    }
}

impl Render for Cockpit {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .child(div().w(px(300.)).h_full().child(self.sidebar.clone()))
            .child(div().flex_1().h_full().child(self.panes.clone()))
    }
}

struct Rig {
    host: Entity<StoreHost>,
    tray: String,
    other: String,
    feature: String,
    _dir: TempDir,
}

impl Rig {
    fn read<R>(&self, cx: &mut VisualTestContext, f: impl FnOnce(&mx_store::Store) -> R) -> R {
        self.host.read_with(cx, |h, _| f(&h.store))
    }

    fn order(&self, cx: &mut VisualTestContext) -> Vec<String> {
        self.read(cx, |s| s.tabs().iter().map(|t| t.id.clone()).collect())
    }
}

fn mount(cx: &mut TestAppContext, keep_open: bool) -> (Rig, &mut VisualTestContext) {
    let r = rig();
    let mut store = r.s;
    let state = r.dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let config = json!({"folders": [{"root": "/repo", "name": "repo", "collapsed": false}]});
    std::fs::write(state.join("config.json"), config.to_string()).unwrap();
    store.load();
    store.set_active_tray(true);
    let folder = Folder::new("/repo", "repo");
    let tray = store.open_shell(&folder, None, None, None, None);
    let other = store.open_shell(&folder, None, None, None, None);
    store.dismiss(&tray);
    if !keep_open {
        store.dismiss(&other);
    }
    store.set_in_active_tray(&tray, true);
    let feature = store.add_feature("/repo", "login", "", FeatureOrHotfixKind::Feature);
    let rt = r.rt;
    let host = cx.update(|cx| {
        Mx::install(mx_theme::default_theme(), cx);
        let host = StoreHost::for_test(store, rt, cx);
        crate::app::ui::install(host.clone(), cx);
        host
    });
    let h = host.clone();
    let (_, vcx) = cx.add_window_view(move |window, cx| Cockpit::new(h, window, cx));
    vcx.run_until_parked();
    (Rig { host, tray, other, feature, _dir: r.dir }, vcx)
}

fn bounds(cx: &mut VisualTestContext, selector: &str) -> Bounds<Pixels> {
    let key: &'static str = Box::leak(selector.to_owned().into_boxed_str());
    cx.debug_bounds(key).unwrap_or_else(|| panic!("{selector} drawn"))
}

fn center(cx: &mut VisualTestContext, selector: &str) -> Point<Pixels> {
    bounds(cx, selector).center()
}

fn drag(cx: &mut VisualTestContext, from: &str, to: &str) {
    let to = center(cx, to);
    drag_to(cx, from, to);
}

fn drag_to(cx: &mut VisualTestContext, from: &str, to: Point<Pixels>) {
    let from = center(cx, from);
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
}

#[gpui::test]
fn a_session_dragged_from_the_tray_onto_the_empty_pane_opens_and_stays_in_the_tray(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx, false);
    drag(cx, &format!("tray-tab:{}", rig.tray), "empty-pane");
    assert!(rig.read(cx, |s| s.is_open(&rig.tray)));
    assert!(rig.read(cx, |s| s.tab(&rig.tray).is_some_and(|t| t.in_active_tray)));
}

#[gpui::test]
fn a_session_dragged_from_the_tray_onto_a_pane_is_placed_beside_it(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx, true);
    let pane = bounds(cx, &format!("pane:{}", rig.other));
    let right_edge = point(pane.right() - px(20.), pane.center().y);
    drag_to(cx, &format!("tray-tab:{}", rig.tray), right_edge);
    assert!(rig.read(cx, |s| s.is_open(&rig.tray) && s.is_open(&rig.other)));
    assert_eq!(rig.read(cx, |s| s.pane_count()), 2);
    assert!(rig.read(cx, |s| s.tab(&rig.tray).is_some_and(|t| t.in_active_tray)));
}

#[gpui::test]
fn releasing_a_tray_drag_over_a_sidebar_row_does_not_reorder(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx, false);
    let before = rig.order(cx);
    drag(cx, &format!("tray-tab:{}", rig.tray), &format!("tab:{}", rig.other));
    assert_eq!(rig.order(cx), before);
}

#[gpui::test]
fn releasing_a_tray_drag_over_a_feature_does_not_assign_it(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx, false);
    drag(cx, &format!("tray-tab:{}", rig.tray), &format!("feature:{}:", rig.feature));
    assert_eq!(rig.read(cx, |s| s.tab(&rig.tray).and_then(|t| t.feature_or_hotfix_id.clone())), None);
}

#[gpui::test]
fn a_sidebar_row_dragged_onto_a_feature_still_joins_it(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx, false);
    drag(cx, &format!("tab:{}", rig.tray), &format!("feature:{}:", rig.feature));
    assert_eq!(rig.read(cx, |s| s.tab(&rig.tray).and_then(|t| t.feature_or_hotfix_id.clone())), Some(rig.feature.clone()));
}

#[gpui::test]
fn clicking_a_tray_row_still_opens_the_session(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx, false);
    assert!(!rig.read(cx, |s| s.is_open(&rig.tray)));
    let at = center(cx, &format!("tray-tab:{}", rig.tray));
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert!(rig.read(cx, |s| s.is_open(&rig.tray)));
}

const GRABS: [f32; 3] = [0.03, 0.5, 0.97];

fn assert_ghost_under_cursor(cx: &mut VisualTestContext, source: &str, ghost: &str, grab: f32) {
    let row = bounds(cx, source);
    let mut at = point(row.left() + row.size.width * grab, row.center().y);
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    for _ in 0..12 {
        at = point(at.x + px(9.), at.y + px(6.));
        cx.simulate_mouse_move(at, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
    }
    let offset = bounds(cx, ghost).origin - at;
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    let (ax, ay) = GHOST_ANCHOR;
    let anchor = point(px(-ax), px(-ay));
    assert!((offset - anchor).magnitude() < 1., "{ghost} grabbed at {grab} of {source}: ghost origin is {offset:?} from the cursor");
}

#[gpui::test]
fn the_drag_ghost_keeps_the_cursor_at_the_same_spot_wherever_a_row_or_pane_header_is_grabbed(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx, true);
    let pane = bounds(cx, &format!("pane:{}", rig.other));
    drag_to(cx, &format!("tray-tab:{}", rig.tray), point(pane.right() - px(20.), pane.center().y));
    for grab in GRABS {
        assert_ghost_under_cursor(cx, &format!("tab:{}", rig.other), "tab-ghost", grab);
        assert_ghost_under_cursor(cx, &format!("pane-header:{}", rig.other), "header-ghost", grab);
    }
}

#[gpui::test]
fn the_drag_ghost_keeps_the_cursor_at_the_same_spot_wherever_a_session_card_is_grabbed(cx: &mut TestAppContext) {
    for grab in GRABS {
        let (rig, cx) = mount(cx, false);
        assert_ghost_under_cursor(cx, &format!("card:{}", rig.other), "card-ghost", grab);
    }
}
