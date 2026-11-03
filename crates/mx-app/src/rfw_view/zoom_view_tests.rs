use std::rc::Rc;

use gpui::{
    Bounds, Modifiers, MouseButton, Pixels, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px, size,
};
use mx_theme::{Typography, default_theme};

use crate::rfw_view::view::RfwView;

const PLAN: &str = r#"ZoomView(maxScale: 6.0, child: FittedBox(fit: "contain",
  child: SizedBox(width: 280.0, height: 140.0, child: Center(child: Text(text: "planta")))))"#;

fn open(cx: &mut TestAppContext) -> &mut VisualTestContext {
    open_plan(cx, PLAN)
}

fn open_plan<'a>(cx: &'a mut TestAppContext, plan: &str) -> &'a mut VisualTestContext {
    let library = format!("import core.widgets;\nimport maestria;\nwidget root = {plan};");
    let (_, vcx) = cx.add_window_view(move |_, _| {
        let mut v = RfwView::new(default_theme().clone(), &Typography::standard(), None, Rc::new(|_, _, _| {}));
        v.set_library(&library);
        v.set_root("root");
        v
    });
    vcx.simulate_resize(size(px(560.0), px(400.0)));
    vcx.run_until_parked();
    vcx
}

fn label(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    cx.run_until_parked();
    cx.debug_bounds("text:planta").expect("the plan text is not painted")
}

fn notches(cx: &mut VisualTestContext, at: gpui::Point<Pixels>, lines: f32) {
    cx.simulate_event(ScrollWheelEvent {
        position: at,
        delta: ScrollDelta::Lines(point(0.0, lines)),
        ..Default::default()
    });
    cx.run_until_parked();
}

fn width(b: Bounds<Pixels>) -> f32 {
    f32::from(b.size.width)
}

#[gpui::test]
fn wheel_zoom_scales_a_fitted_child_on_both_axes_around_the_pointer(cx: &mut TestAppContext) {
    let cx = open(cx);
    let fit = label(cx);
    let at = fit.center();
    notches(cx, at, 3.0);
    let one = label(cx);
    let k = width(one) / width(fit);
    assert!((1.2..1.4).contains(&k), "one notch should zoom ~30%, got {k} ({fit:?} -> {one:?})");
    assert!((f32::from(one.center().x) - f32::from(at.x)).abs() < 3.0, "{at:?} {one:?}");
    assert!((f32::from(one.center().y) - f32::from(at.y)).abs() < 3.0, "{at:?} {one:?}");
    for _ in 0..4 {
        notches(cx, at, 3.0);
    }
    let deep = label(cx);
    assert!(width(deep) / width(fit) > 3.0, "{fit:?} -> {deep:?}");
    assert!(f32::from(deep.origin.x) > 0.0 && f32::from(deep.origin.x + deep.size.width) < 560.0, "{deep:?}");
    for _ in 0..12 {
        notches(cx, at, -3.0);
    }
    let back = label(cx);
    assert!((width(back) - width(fit)).abs() < 1.0, "{fit:?} -> {back:?}");
    assert!((back.origin.x - fit.origin.x).abs() < px(1.0), "{fit:?} -> {back:?}");
}

#[test]
fn focus_command_animates_once_per_rev_and_never_counts_as_a_user_move() {
    use std::time::{Duration, Instant};
    let bounds = Bounds::new(point(px(0.), px(0.)), size(px(1000.), px(800.)));
    let mut st = super::ZoomState { bounds: Some(bounds), ..Default::default() };
    let t0 = Instant::now();
    let rect = Some((0.5, 0.5, 0.25, 0.25));
    st.command(Some("focus"), rect, 0, 6.0, t0);
    assert!(!st.animating(), "the mount revision is ignored");
    st.command(Some("focus"), rect, 1, 6.0, t0);
    assert!(st.animating());
    let end = st.current(t0 + Duration::from_secs(1));
    assert!((end.scale - 3.6).abs() < 1e-9, "{end:?}");
    assert_eq!(st.settled_move(), None, "programmatic commands never report onMove");
    st.command(Some("focus"), rect, 1, 6.0, t0);
    assert!(!st.animating(), "same rev does nothing");
    assert!(!st.drag(point(px(0.), px(0.))));
    st.wheel(point(px(500.), px(400.)), -100.0, 6.0);
    let m = st.settled_move().expect("a user wheel reports");
    assert_eq!(m, st.matrix);
    assert_eq!(st.settled_move(), None);
}

#[test]
fn a_programmatic_command_discards_a_pending_user_move() {
    use std::time::{Duration, Instant};
    let bounds = Bounds::new(point(px(0.), px(0.)), size(px(1000.), px(800.)));
    let mut st = super::ZoomState { bounds: Some(bounds), ..Default::default() };
    let t0 = Instant::now();
    st.command(Some("focus"), None, 0, 6.0, t0);
    st.wheel(point(px(500.), px(400.)), -100.0, 6.0);
    st.command(Some("focus"), Some((0.5, 0.5, 0.25, 0.25)), 1, 6.0, t0);
    st.current(t0 + Duration::from_secs(1));
    assert_eq!(st.settled_move(), None);
}

const MIDDLE_ONLY: &str = r#"ZoomView(maxScale: 6.0, panButtons: ["middle"], child: FittedBox(fit: "contain",
  child: SizedBox(width: 280.0, height: 140.0, child: Center(child: Text(text: "planta")))))"#;

fn drag(cx: &mut VisualTestContext, button: MouseButton, from: gpui::Point<Pixels>) -> Bounds<Pixels> {
    let none = Modifiers::default();
    cx.simulate_mouse_down(from, button, none);
    cx.simulate_mouse_move(from + point(px(40.), px(0.)), button, none);
    cx.simulate_mouse_up(from + point(px(40.), px(0.)), button, none);
    label(cx)
}

#[gpui::test]
fn pan_buttons_gate_which_button_drags_and_space_always_pans_with_left(cx: &mut TestAppContext) {
    let cx = open_plan(cx, MIDDLE_ONLY);
    let at = label(cx).center();
    for _ in 0..3 {
        notches(cx, at, 3.0);
    }
    let start = label(cx);
    let left = drag(cx, MouseButton::Left, at);
    assert_eq!(left.origin.x, start.origin.x, "left must not pan with panButtons: [middle]");
    let middle = drag(cx, MouseButton::Middle, at);
    assert!(f32::from(middle.origin.x - start.origin.x) > 30.0, "{start:?} -> {middle:?}");
    let space = gpui::Keystroke::parse("space").unwrap();
    cx.simulate_event(gpui::KeyDownEvent { keystroke: space.clone(), is_held: false, ime_commit: false });
    let before = label(cx);
    let spaced = drag(cx, MouseButton::Left, at);
    assert!(f32::from(spaced.origin.x - before.origin.x) > 30.0, "{before:?} -> {spaced:?}");
    cx.simulate_event(gpui::KeyUpEvent { keystroke: space });
    let after = label(cx);
    let released = drag(cx, MouseButton::Left, at);
    assert_eq!(released.origin.x, after.origin.x, "releasing space ends the left-pan");
}

#[gpui::test]
fn default_pan_buttons_keep_the_left_drag(cx: &mut TestAppContext) {
    let cx = open(cx);
    let at = label(cx).center();
    for _ in 0..3 {
        notches(cx, at, 3.0);
    }
    let start = label(cx);
    let left = drag(cx, MouseButton::Left, at);
    assert!(f32::from(left.origin.x - start.origin.x) > 30.0, "{start:?} -> {left:?}");
}

#[gpui::test]
fn space_is_released_when_the_view_loses_focus(cx: &mut TestAppContext) {
    let cx = open_plan(cx, MIDDLE_ONLY);
    cx.update(|window, _| window.activate_window());
    let at = label(cx).center();
    for _ in 0..3 {
        notches(cx, at, 3.0);
    }
    drag(cx, MouseButton::Middle, at);
    let space = gpui::Keystroke::parse("space").unwrap();
    cx.simulate_event(gpui::KeyDownEvent { keystroke: space, is_held: false, ime_commit: false });
    cx.update(|window, app| {
        let elsewhere = app.focus_handle();
        window.focus(&elsewhere);
        window.refresh();
    });
    cx.run_until_parked();
    let before = label(cx);
    let after = drag(cx, MouseButton::Left, at);
    assert_eq!(after.origin.x, before.origin.x, "a stale space must not pan with panButtons: [middle]");
}

#[gpui::test]
fn a_focusable_child_keeps_focus_when_clicked(cx: &mut TestAppContext) {
    let plan = r#"ZoomView(maxScale: 6.0, child: Column(children: [TextInput(id: "chat")]))"#;
    let library = format!("import core.widgets;\nimport maestria;\nwidget root = {plan};");
    let (view, cx) = cx.add_window_view(move |_, _| {
        let mut v = RfwView::new(default_theme().clone(), &Typography::standard(), None, Rc::new(|_, _, _| {}));
        v.set_library(&library);
        v.set_root("root");
        v
    });
    cx.simulate_resize(size(px(560.0), px(400.0)));
    cx.run_until_parked();
    let field = view.read_with(cx, |v, _| v.text_field("chat")).expect("no TextInput");
    let at = point(px(100.), px(20.));
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    let focused = cx.update(|window, app| field.read(app).focus.is_focused(window));
    assert!(focused, "the zoom view must not take focus from the TextInput under the pointer");
}

fn zoomed_state() -> super::ZoomState {
    let bounds = Bounds::new(point(px(0.), px(0.)), size(px(1000.), px(800.)));
    let mut st = super::ZoomState { bounds: Some(bounds), ..Default::default() };
    st.wheel(point(px(500.), px(400.)), -100.0, 6.0);
    st
}

#[test]
fn drag_without_grab_never_pans() {
    let mut st = zoomed_state();
    let before = st.matrix;
    for p in [point(px(10.), px(10.)), point(px(30.), px(25.)), point(px(60.), px(50.))] {
        assert!(!st.drag(p));
        assert_eq!(st.matrix, before);
    }
}

#[test]
fn drag_after_grab_pans() {
    let mut st = zoomed_state();
    let before = st.matrix;
    st.grab = Some(point(px(100.), px(100.)));
    assert!(st.drag(point(px(110.), px(105.))));
    assert!((st.matrix.x - before.x - 10.0).abs() < 1e-9, "{:?} vs {before:?}", st.matrix);
    assert!((st.matrix.y - before.y - 5.0).abs() < 1e-9, "{:?} vs {before:?}", st.matrix);
}
