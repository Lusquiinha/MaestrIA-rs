use gpui::{
    Entity, FocusHandle, Modifiers, MouseButton, NavigationDirection, TestAppContext, VisualTestContext, px, size,
};
use mx_core::layout::DropSide;
use mx_core::model::Folder;
use serde_json::{Value, json};
use tempfile::TempDir;

use crate::app::host::StoreHost;
use crate::app::root::Root;
use crate::app::theme::Mx;
use crate::test_store::rig;

const BACK: MouseButton = MouseButton::Navigate(NavigationDirection::Back);
const FORWARD: MouseButton = MouseButton::Navigate(NavigationDirection::Forward);

struct Window2 {
    host: Entity<StoreHost>,
    a: String,
    b: String,
    _dir: TempDir,
}

impl Window2 {
    fn focused(&self, cx: &mut VisualTestContext) -> Option<String> {
        self.host.read_with(cx, |h, _| h.store.focused_tab_id())
    }
}

fn keyboard_focus(cx: &mut VisualTestContext) -> Option<FocusHandle> {
    cx.update(|window, cx| window.focused(cx))
}

fn mount(cx: &mut TestAppContext, shortcuts: Value) -> (Window2, &mut VisualTestContext) {
    let r = rig();
    let mut store = r.s;
    let folder = Folder::new("/repo", "repo");
    let a = store.open_shell(&folder, None, None, None, None);
    let b = store.open_shell_toward(&folder, None, DropSide::Right);
    store.focus_pane(&a);
    store.set_shortcuts(shortcuts.as_object().unwrap().clone());
    let rt = r.rt;
    let host = cx.update(|cx| {
        Mx::install(mx_theme::default_theme(), cx);
        let host = StoreHost::for_test(store, rt, cx);
        crate::app::ui::install(host.clone(), cx);
        host
    });
    let h = host.clone();
    let (_, vcx) = cx.add_window_view(move |window, cx| Root::new(h, window, cx));
    vcx.simulate_resize(size(px(1400.), px(900.)));
    vcx.run_until_parked();
    (Window2 { host, a, b, _dir: r.dir }, vcx)
}

fn press(cx: &mut VisualTestContext, pane: &str, button: MouseButton, modifiers: Modifiers) {
    press_on(cx, Box::leak(format!("pane:{pane}").into_boxed_str()), button, modifiers);
}

fn press_on(cx: &mut VisualTestContext, selector: &'static str, button: MouseButton, modifiers: Modifiers) {
    let at = cx.debug_bounds(selector).unwrap_or_else(|| panic!("no element {selector}")).center();
    cx.simulate_mouse_down(at, button, modifiers);
    cx.simulate_mouse_up(at, button, modifiers);
    cx.run_until_parked();
}

#[gpui::test]
fn a_side_button_over_another_pane_acts_on_the_focused_one(cx: &mut TestAppContext) {
    let (w, cx) = mount(cx, json!({"next-pane": ["mouse5"]}));
    assert_eq!(w.focused(cx).as_deref(), Some(w.a.as_str()));
    press(cx, &w.b, FORWARD, Modifiers::none());
    assert_eq!(w.focused(cx).as_deref(), Some(w.b.as_str()));
    press(cx, &w.b, FORWARD, Modifiers::none());
    assert_eq!(w.focused(cx).as_deref(), Some(w.a.as_str()));
}

#[gpui::test]
fn a_side_button_needs_exactly_its_modifiers(cx: &mut TestAppContext) {
    let (w, cx) = mount(cx, json!({"next-pane": ["ctrl+mouse5"]}));
    press(cx, &w.b, FORWARD, Modifiers::none());
    assert_eq!(w.focused(cx).as_deref(), Some(w.a.as_str()));
    press(cx, &w.b, FORWARD, Modifiers::control());
    assert_eq!(w.focused(cx).as_deref(), Some(w.b.as_str()));
}

#[gpui::test]
fn an_unbound_side_button_does_not_move_the_focus(cx: &mut TestAppContext) {
    let (w, cx) = mount(cx, json!({}));
    let before = keyboard_focus(cx);
    press(cx, &w.b, FORWARD, Modifiers::none());
    assert_eq!(w.focused(cx).as_deref(), Some(w.a.as_str()));
    assert!(keyboard_focus(cx) == before, "the keyboard stays where it was");
}
