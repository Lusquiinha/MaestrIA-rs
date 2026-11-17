use std::rc::Rc;

use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, ScrollDelta, ScrollWheelEvent, Styled, TestAppContext,
    Window, div, point, px, size,
};
use mx_theme::{Typography, default_theme};

use super::Surface;
use crate::rfw_view::BlocksMode;
use crate::rfw_view::RfwView;

/// A área dos painéis, com a tela de um plugin dentro e um contador que muda
/// sozinho (o spinner de uma sessão, outra aba).
struct Panes {
    surface: Surface,
    ticks: usize,
}

impl Render for Panes {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let body = div().flex_1().child(self.surface.element());
        div().size_full().flex().flex_col().child(format!("{}", self.ticks)).child(body)
    }
}

fn open(cx: &mut TestAppContext) -> (Entity<Panes>, Entity<RfwView>, &mut gpui::VisualTestContext) {
    let lines: String = (0..300).map(|i| format!("Text(text: \"linha {i}\"),")).collect();
    open_with(cx, format!("import core.widgets;\nwidget root = ListView(children: [{lines}]);"))
}

fn open_with(
    cx: &mut TestAppContext,
    library: String,
) -> (Entity<Panes>, Entity<RfwView>, &mut gpui::VisualTestContext) {
    let mut rfw = None;
    let (panes, vcx) = cx.add_window_view(|_, cx| {
        let view = cx.new(|_| {
            let mut v = RfwView::new(default_theme().clone(), &Typography::standard(), None, Rc::new(|_, _, _| {}));
            v.set_library(&library);
            v
        });
        rfw = Some(view.clone());
        let mut surface = Surface::new("aba", BlocksMode::Bare);
        surface.rfw = Some(view);
        Panes { surface, ticks: 0 }
    });
    vcx.simulate_resize(size(px(800.0), px(600.0)));
    vcx.run_until_parked();
    (panes, rfw.unwrap(), vcx)
}

#[gpui::test]
fn the_panes_redrawing_around_a_plugin_screen_do_not_rebuild_it(cx: &mut TestAppContext) {
    let (panes, rfw, cx) = open(cx);
    let before = rfw.read_with(cx, |v, _| v.renders);
    for _ in 0..10 {
        panes.update(cx, |p, cx| {
            p.ticks += 1;
            cx.notify();
        });
        cx.run_until_parked();
    }
    assert_eq!(rfw.read_with(cx, |v, _| v.renders), before);
}

#[gpui::test]
fn a_plugin_screen_that_changes_is_drawn_again(cx: &mut TestAppContext) {
    let (_, rfw, cx) = open(cx);
    let before = rfw.read_with(cx, |v, _| v.renders);
    rfw.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    assert_eq!(rfw.read_with(cx, |v, _| v.renders), before + 1);
}

#[gpui::test]
fn a_resized_window_draws_the_plugin_screen_again(cx: &mut TestAppContext) {
    let (_, rfw, cx) = open(cx);
    let before = rfw.read_with(cx, |v, _| v.renders);
    cx.simulate_resize(size(px(1000.0), px(600.0)));
    cx.run_until_parked();
    assert!(rfw.read_with(cx, |v, _| v.renders) > before);
}

#[gpui::test]
fn scrolling_a_plugin_screen_does_not_evaluate_it_again(cx: &mut TestAppContext) {
    let (_, rfw, cx) = open(cx);
    let (renders, evaluations) = rfw.read_with(cx, |v, _| (v.renders, v.evaluations));
    for _ in 0..5 {
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(200.0), px(300.0)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-40.0))),
            ..Default::default()
        });
        cx.run_until_parked();
    }
    assert!(rfw.read_with(cx, |v, _| v.renders) > renders, "a rolagem redesenha a tela");
    assert_eq!(rfw.read_with(cx, |v, _| v.evaluations), evaluations);
}

fn scroll_by(cx: &mut gpui::VisualTestContext, dy: f32) {
    cx.simulate_event(ScrollWheelEvent {
        position: point(px(200.0), px(300.0)),
        delta: ScrollDelta::Pixels(point(px(0.0), px(dy))),
        ..Default::default()
    });
    cx.run_until_parked();
}

#[gpui::test]
fn a_long_list_builds_only_what_shows_and_still_reaches_the_end(cx: &mut TestAppContext) {
    let (_, _, cx) = open(cx);
    // O primeiro quadro mede tudo; o próximo já é só a janela.
    scroll_by(cx, -1.0);
    let shown = crate::rfw_view::virtual_shown_now();
    assert!(shown.iter().any(|r| r.start == 0 && r.len() < 150), "só o começo: {shown:?}");
    for _ in 0..200 {
        scroll_by(cx, -400.0);
    }
    let shown = crate::rfw_view::virtual_shown_now();
    assert!(shown.iter().any(|r| r.end == 300 && r.start > 150), "o fim: {shown:?}");
}

#[gpui::test]
fn a_long_list_with_anchors_stays_whole(cx: &mut TestAppContext) {
    let rows: String = (0..300).map(|i| format!("Anchor(id: \"a{i}\", child: Text(text: \"linha {i}\")),")).collect();
    let (_, _, cx) = open_with(cx, format!("import core.widgets;\nimport maestria;\nwidget root = ListView(children: [{rows}]);"));
    scroll_by(cx, -1.0);
    // A lista virtual desistiu no primeiro quadro: ficou com tudo montado.
    let shown = crate::rfw_view::virtual_shown_now();
    assert!(!shown.is_empty() && shown.iter().all(|r| r.len() == 300), "{shown:?}");
}

#[gpui::test]
fn new_plugin_data_evaluates_the_screen_again(cx: &mut TestAppContext) {
    let (_, rfw, cx) = open(cx);
    let before = rfw.read_with(cx, |v, _| v.evaluations);
    rfw.update(cx, |v, cx| {
        let data = serde_json::json!({ "count": 1 });
        v.merge_data(data.as_object().unwrap());
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(rfw.read_with(cx, |v, _| v.evaluations), before + 1);
}
