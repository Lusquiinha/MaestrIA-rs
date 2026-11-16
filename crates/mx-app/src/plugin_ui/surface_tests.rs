use std::rc::Rc;

use gpui::{AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px, size};
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
    let library = format!("import core.widgets;\nwidget root = ListView(children: [{lines}]);");
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
