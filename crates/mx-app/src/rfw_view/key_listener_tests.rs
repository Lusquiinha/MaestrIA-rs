use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AppContext, Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement, KeyContext, KeyDownEvent, KeyUpEvent,
    Keystroke, Modifiers, MouseButton, ParentElement, Pixels, Render, Styled, TestAppContext, VisualTestContext, Window,
    div, point, px, size,
};
use mx_plugins::chord::KeyPress;
use mx_theme::{Typography, default_theme};

use super::{chords, claimed, matching};
use crate::keymap::press_of;
use crate::rfw_view::view::RfwView;

const KEYS: &str = r#"["ctrl+c", "escape", "ctrl+0", "ctrl+=", "ctrl+-", "ctrl+plus"]"#;

fn stroke(key: &str, modifiers: Modifiers) -> Keystroke {
    Keystroke { modifiers, key: key.into(), key_char: None }
}

fn press(key: &str, modifiers: Modifiers) -> KeyPress {
    press_of(&stroke(key, modifiers)).expect("not a key")
}

fn declared() -> Vec<String> {
    serde_json::from_str(KEYS).unwrap()
}

#[test]
fn chords_match_with_exact_modifiers() {
    let c = chords(&declared());
    assert_eq!(c.len(), 6);
    assert_eq!(matching(&c, &press("c", Modifiers::control())).as_deref(), Some("ctrl+c"));
    assert_eq!(matching(&c, &press("c", Modifiers::control_shift())), None);
    assert_eq!(matching(&c, &press("c", Modifiers::none())), None);
    assert_eq!(matching(&c, &press("escape", Modifiers::none())).as_deref(), Some("escape"));
    assert_eq!(matching(&c, &press("escape", Modifiers::control())), None);
    assert_eq!(matching(&c, &press("0", Modifiers::control())).as_deref(), Some("ctrl+0"));
    assert_eq!(matching(&c, &press("=", Modifiers::control())).as_deref(), Some("ctrl+="));
    assert_eq!(matching(&c, &press("-", Modifiers::control())).as_deref(), Some("ctrl+-"));
    assert_eq!(matching(&c, &press("+", Modifiers::control())).as_deref(), Some("ctrl+plus"));
    let meta = chords(&["meta+c".to_owned(), "bogus+c".to_owned(), String::new()]);
    assert_eq!(meta.len(), 1);
    assert_eq!(matching(&meta, &press("c", Modifiers::command())).as_deref(), Some("meta+c"));
    assert_eq!(matching(&meta, &press("c", Modifiers::control())), None);
}

fn plugin_keys(keys: &str) -> KeyContext {
    let mut ctx = KeyContext::default();
    ctx.add("PluginKeys");
    ctx.set("pluginKeys", keys.to_owned());
    ctx
}

fn named(name: &str) -> KeyContext {
    KeyContext::parse(name).unwrap()
}

#[test]
fn the_root_defers_only_to_a_focused_listener_outside_text_fields() {
    let ctrl_eq = press("=", Modifiers::control());
    let listener = plugin_keys("ctrl+c ctrl+=");
    assert_eq!(claimed(&[named("Root"), listener.clone()], &ctrl_eq).as_deref(), Some("ctrl+="));
    assert_eq!(claimed(&[named("Root"), named("Terminal")], &ctrl_eq), None);
    assert_eq!(claimed(&[named("Root"), listener.clone(), named("PluginTextInput")], &ctrl_eq), None);
    assert_eq!(claimed(&[named("Root"), listener.clone(), named("CodeEditor")], &ctrl_eq), None);
    assert_eq!(claimed(&[named("Root"), listener.clone(), named("TextField")], &ctrl_eq), None);
    assert_eq!(claimed(&[named("Root"), listener], &press("0", Modifiers::control())), None);
    let outer = plugin_keys("ctrl+0");
    let inner = plugin_keys("escape");
    assert_eq!(claimed(&[outer, inner], &press("0", Modifiers::control())).as_deref(), Some("ctrl+0"));
}

type Seen = Rc<RefCell<Vec<serde_json::Map<String, serde_json::Value>>>>;

const CANVAS: &str = r#"ZoomView(maxScale: 6.0, panButtons: ["middle"], child: FittedBox(fit: "contain",
  child: SizedBox(width: 280.0, height: 140.0, child: Center(child: Text(text: "planta")))))"#;

fn rfw(plan: &str, seen: &Seen, cx: &mut gpui::App) -> Entity<RfwView> {
    let library = format!(
        "import core.widgets;\nimport maestria;\nwidget root = KeyListener(keys: {KEYS}, onKey: event \"act\" {{a: \"key\"}}, child: {plan});"
    );
    let seen = seen.clone();
    cx.new(move |_| {
        let sink = Rc::new(move |a: super::super::ViewAction, _: &mut Window, _: &mut gpui::App| {
            seen.borrow_mut().push(a.values);
        });
        let mut v = RfwView::new(default_theme().clone(), &Typography::standard(), None, sink);
        v.set_library(&library);
        v.set_root("root");
        v
    })
}

struct Split {
    term: FocusHandle,
    rfw: Entity<RfwView>,
}

impl Render for Split {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let term = div().w(px(200.)).h_full().key_context("Terminal").track_focus(&self.term).on_key_down(|_, _, _| {});
        div().size_full().flex().flex_row().child(term).child(div().flex_1().h_full().child(self.rfw.clone()))
    }
}

fn open<'a>(cx: &'a mut TestAppContext, plan: &str, seen: &Seen) -> (Entity<Split>, &'a mut VisualTestContext) {
    let plan = plan.to_owned();
    let seen = seen.clone();
    let (split, vcx) = cx.add_window_view(move |_, cx| Split { term: cx.focus_handle(), rfw: rfw(&plan, &seen, cx) });
    vcx.simulate_resize(size(px(760.0), px(400.0)));
    vcx.update(|window, _| window.activate_window());
    vcx.run_until_parked();
    (split, vcx)
}

fn key(cx: &mut VisualTestContext, key: &str, modifiers: Modifiers) {
    cx.simulate_event(KeyDownEvent { keystroke: stroke(key, modifiers), is_held: false, ime_commit: false });
    cx.simulate_event(KeyUpEvent { keystroke: stroke(key, modifiers) });
    cx.run_until_parked();
}

fn click(cx: &mut VisualTestContext, at: gpui::Point<Pixels>) {
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
}

fn keys_seen(seen: &Seen) -> Vec<String> {
    seen.borrow().iter().map(|v| format!("{}:{}", v["a"].as_str().unwrap(), v["key"].as_str().unwrap())).collect()
}

#[gpui::test]
fn a_focused_canvas_gets_its_keys_and_a_focused_terminal_never_does(cx: &mut TestAppContext) {
    let seen = Seen::default();
    let (split, cx) = open(cx, CANVAS, &seen);
    let term = split.read_with(cx, |s, _| s.term.clone());
    cx.update(|window, _| window.focus(&term));
    cx.run_until_parked();
    for (k, m) in [("c", Modifiers::control()), ("escape", Modifiers::none()), ("=", Modifiers::control())] {
        key(cx, k, m);
    }
    assert!(seen.borrow().is_empty(), "a focused terminal must keep its keys: {:?}", keys_seen(&seen));
    click(cx, point(px(480.), px(200.)));
    key(cx, "c", Modifiers::control());
    key(cx, "c", Modifiers::control_shift());
    key(cx, "x", Modifiers::control());
    key(cx, "escape", Modifiers::none());
    key(cx, "-", Modifiers::control());
    assert_eq!(keys_seen(&seen), ["key:ctrl+c", "key:escape", "key:ctrl+-"]);
}

#[gpui::test]
fn a_focused_text_input_keeps_ctrl_c_and_escape(cx: &mut TestAppContext) {
    let seen = Seen::default();
    let plan = r#"Column(children: [TextInput(id: "chat")])"#;
    let (split, cx) = open(cx, plan, &seen);
    let rfw = split.read_with(cx, |s, _| s.rfw.clone());
    let field = rfw.read_with(cx, |v, _| v.text_field("chat")).expect("no TextInput");
    click(cx, point(px(300.), px(20.)));
    assert!(cx.update(|window, app| field.read(app).focus.is_focused(window)), "the click must focus the field");
    key(cx, "c", Modifiers::control());
    key(cx, "escape", Modifiers::none());
    key(cx, "=", Modifiers::control());
    assert!(seen.borrow().is_empty(), "{:?}", keys_seen(&seen));
}

fn label(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    cx.run_until_parked();
    cx.debug_bounds("text:planta").expect("the plan text is not painted")
}

#[gpui::test]
fn space_drag_pans_after_a_left_click_inside_the_listener(cx: &mut TestAppContext) {
    let seen = Seen::default();
    let (_, cx) = open(cx, CANVAS, &seen);
    let at = label(cx).center();
    for _ in 0..3 {
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: at,
            delta: gpui::ScrollDelta::Lines(point(0.0, 3.0)),
            ..Default::default()
        });
        cx.run_until_parked();
    }
    click(cx, at);
    let before = label(cx);
    cx.simulate_event(KeyDownEvent { keystroke: stroke("space", Modifiers::none()), is_held: false, ime_commit: false });
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(at + point(px(40.), px(0.)), MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at + point(px(40.), px(0.)), MouseButton::Left, Modifiers::none());
    let after = label(cx);
    cx.simulate_event(KeyUpEvent { keystroke: stroke("space", Modifiers::none()) });
    assert!(f32::from(after.origin.x - before.origin.x) > 30.0, "{before:?} -> {after:?}");
}

#[gpui::test]
fn a_press_that_a_child_stops_still_focuses_the_listener(cx: &mut TestAppContext) {
    let seen = Seen::default();
    let plan = r#"Pressable(menu: [{label: "apagar", action: "rm"}], onMenu: event "act" {a: "menu"},
  child: SizedBox(width: 200.0, height: 100.0, child: Text(text: "prancheta")))"#;
    let (split, cx) = open(cx, plan, &seen);
    let term = split.read_with(cx, |s, _| s.term.clone());
    cx.update(|window, _| window.focus(&term));
    cx.run_until_parked();
    let at = cx.debug_bounds("text:prancheta").expect("the board text is not painted").center();
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
    assert!(!cx.update(|window, app| term.contains_focused(window, app)), "the press must move focus off the terminal");
    key(cx, "c", Modifiers::control());
    assert_eq!(keys_seen(&seen), ["key:ctrl+c"]);
}

struct Pane {
    pane: FocusHandle,
    rfw: Entity<RfwView>,
}

impl Render for Pane {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().track_focus(&self.pane).child(self.rfw.clone())
    }
}

#[gpui::test]
fn first_click_keeps_focus_inside_listener(cx: &mut TestAppContext) {
    let seen = Seen::default();
    let (pane, cx) = cx.add_window_view({
        let seen = seen.clone();
        move |_, cx| Pane { pane: cx.focus_handle(), rfw: rfw(CANVAS, &seen, cx) }
    });
    cx.simulate_resize(size(px(560.0), px(400.0)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let handle = pane.read_with(cx, |p, _| p.pane.clone());
    let at = label(cx).center();
    click(cx, at);
    cx.update(|window, app| crate::panes::focus_unless_inside(&handle, window, app));
    cx.run_until_parked();
    assert!(cx.update(|window, app| handle.contains_focused(window, app)), "focus must stay in the pane");
    assert!(!cx.update(|window, _| handle.is_focused(window)), "the pane wrapper must not take focus from the listener");
    key(cx, "c", Modifiers::control());
    assert_eq!(keys_seen(&seen), ["key:ctrl+c"]);
}

#[gpui::test]
fn listener_space_cleared_on_window_deactivate(cx: &mut TestAppContext) {
    let seen = Seen::default();
    let (split, cx) = open(cx, CANVAS, &seen);
    let at = label(cx).center();
    for _ in 0..3 {
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: at,
            delta: gpui::ScrollDelta::Lines(point(0.0, 3.0)),
            ..Default::default()
        });
        cx.run_until_parked();
    }
    click(cx, at);
    cx.simulate_event(KeyDownEvent { keystroke: stroke("space", Modifiers::none()), is_held: false, ime_commit: false });
    let rfw = split.read_with(cx, |s, _| s.rfw.clone());
    let held = |cx: &mut VisualTestContext| {
        let mut held = false;
        rfw.read_with(cx, |v, _| v.widget_states().keys.each(|k| held |= k.space));
        held
    };
    assert!(held(cx), "space must be held before the window loses activation");
    rfw.read_with(cx, |v, _| v.widget_states().keys.each(|k| k.blur = None));
    cx.deactivate_window();
    assert!(!held(cx), "deactivating the window must release space even without a focus-out");
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let before = label(cx);
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(at + point(px(40.), px(0.)), MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at + point(px(40.), px(0.)), MouseButton::Left, Modifiers::none());
    let after = label(cx);
    assert_eq!(after.origin.x, before.origin.x, "a space released outside the window must not keep panning");
}
