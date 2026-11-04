use std::cell::Cell;

use gpui::{
    AnyElement, CursorStyle, Decorations, Div, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, ResizeEdge, Stateful, StatefulInteractiveElement, Styled, Tiling, Window,
    WindowControls, div, px, svg,
};

use crate::app::theme::Mx;
use crate::app::tooltip::tip;

pub const STRIP_HEIGHT: f32 = 30.;
const EDGE: f32 = 5.;
const CORNER: f32 = 12.;
const BUTTON: f32 = 22.;
const GLYPH: f32 = 16.;

thread_local! {
    static PRESSED: Cell<bool> = const { Cell::new(false) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Minimize,
    Maximize,
    Close,
}

impl Button {
    fn id(self) -> &'static str {
        match self {
            Button::Minimize => "window-minimize",
            Button::Maximize => "window-maximize",
            Button::Close => "window-close",
        }
    }

    pub fn glyph(self, maximized: bool) -> &'static str {
        match self {
            Button::Minimize => "glyphs/window_minimize.svg",
            Button::Maximize if maximized => "glyphs/window_restore.svg",
            Button::Maximize => "glyphs/window_maximize.svg",
            Button::Close => "glyphs/window_close.svg",
        }
    }

    pub fn label(self, maximized: bool) -> &'static str {
        match self {
            Button::Minimize => "Minimizar",
            Button::Maximize if maximized => "Restaurar",
            Button::Maximize => "Maximizar",
            Button::Close => "Fechar",
        }
    }
}

pub fn buttons(controls: WindowControls) -> Vec<Button> {
    let mut out = Vec::with_capacity(3);
    if controls.minimize {
        out.push(Button::Minimize);
    }
    if controls.maximize {
        out.push(Button::Maximize);
    }
    out.push(Button::Close);
    out
}

pub fn resize_edges(tiling: Tiling, maximized: bool) -> Vec<ResizeEdge> {
    if maximized {
        return Vec::new();
    }
    let free = |e: ResizeEdge| match e {
        ResizeEdge::Top => !tiling.top,
        ResizeEdge::Bottom => !tiling.bottom,
        ResizeEdge::Left => !tiling.left,
        ResizeEdge::Right => !tiling.right,
        ResizeEdge::TopLeft => !tiling.top && !tiling.left,
        ResizeEdge::TopRight => !tiling.top && !tiling.right,
        ResizeEdge::BottomLeft => !tiling.bottom && !tiling.left,
        ResizeEdge::BottomRight => !tiling.bottom && !tiling.right,
    };
    [
        ResizeEdge::Top,
        ResizeEdge::Bottom,
        ResizeEdge::Left,
        ResizeEdge::Right,
        ResizeEdge::TopLeft,
        ResizeEdge::TopRight,
        ResizeEdge::BottomLeft,
        ResizeEdge::BottomRight,
    ]
    .into_iter()
    .filter(|e| free(*e))
    .collect()
}

pub fn client_tiling(window: &Window) -> Option<Tiling> {
    match window.window_decorations() {
        Decorations::Client { tiling } => Some(tiling),
        Decorations::Server => None,
    }
}

pub fn drag_area(el: Stateful<Div>) -> Stateful<Div> {
    el.on_mouse_down(MouseButton::Left, |_, _, _| PRESSED.set(true))
        .on_mouse_up(MouseButton::Left, |_, _, _| PRESSED.set(false))
        .on_mouse_move(|ev, window, _| {
            if ev.pressed_button == Some(MouseButton::Left) && PRESSED.replace(false) {
                window.start_window_move();
            }
        })
        .on_mouse_down(MouseButton::Right, |ev, window, _| window.show_window_menu(ev.position))
        .on_click(|ev, window, _| {
            if ev.click_count() == 2 {
                window.zoom_window();
            }
        })
}

pub fn controls(window: &Window, mx: &Mx) -> Div {
    let maximized = window.is_maximized();
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(6.))
        .children(buttons(window.window_controls()).into_iter().map(|b| button(b, maximized, mx)))
}

fn button(b: Button, maximized: bool, mx: &Mx) -> impl IntoElement {
    div()
        .id(b.id())
        .size(px(BUTTON))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(mx.bg_hover.opacity(0.5))
        .hover(|s| s.bg(mx.bg_hover))
        .active(|s| s.bg(mx.bg_active))
        .tooltip(tip(b.label(maximized)))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            match b {
                Button::Minimize => window.minimize_window(),
                Button::Maximize => window.zoom_window(),
                Button::Close => window.remove_window(),
            }
        })
        .child(glyph(b.glyph(maximized), mx.fg_dim))
}

fn glyph(path: &'static str, color: Hsla) -> impl IntoElement {
    svg().path(path).flex_none().size(px(GLYPH)).text_color(color)
}

pub fn strip(window: &Window, mx: &Mx) -> impl IntoElement {
    drag_area(div().id("window-strip"))
        .h(px(STRIP_HEIGHT))
        .w_full()
        .flex_none()
        .flex()
        .items_center()
        .justify_end()
        .pr(px(8.))
        .child(controls(window, mx))
}

pub fn resize_handles(window: &Window, tiling: Tiling) -> Vec<AnyElement> {
    resize_edges(tiling, window.is_maximized())
        .into_iter()
        .map(handle)
        .collect()
}

fn handle(edge: ResizeEdge) -> AnyElement {
    let d = div().absolute();
    let d = match edge {
        ResizeEdge::Top => d.top_0().left(px(CORNER)).right(px(CORNER)).h(px(EDGE)),
        ResizeEdge::Bottom => d.bottom_0().left(px(CORNER)).right(px(CORNER)).h(px(EDGE)),
        ResizeEdge::Left => d.left_0().top(px(CORNER)).bottom(px(CORNER)).w(px(EDGE)),
        ResizeEdge::Right => d.right_0().top(px(CORNER)).bottom(px(CORNER)).w(px(EDGE)),
        ResizeEdge::TopLeft => d.top_0().left_0().size(px(CORNER)),
        ResizeEdge::TopRight => d.top_0().right_0().size(px(CORNER)),
        ResizeEdge::BottomLeft => d.bottom_0().left_0().size(px(CORNER)),
        ResizeEdge::BottomRight => d.bottom_0().right_0().size(px(CORNER)),
    };
    d.cursor(cursor(edge))
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            window.start_window_resize(edge);
        })
        .into_any_element()
}

fn cursor(edge: ResizeEdge) -> CursorStyle {
    match edge {
        ResizeEdge::Top | ResizeEdge::Bottom => CursorStyle::ResizeUpDown,
        ResizeEdge::Left | ResizeEdge::Right => CursorStyle::ResizeLeftRight,
        ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorStyle::ResizeUpLeftDownRight,
        ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controls(minimize: bool, maximize: bool) -> WindowControls {
        WindowControls { fullscreen: true, maximize, minimize, window_menu: true }
    }

    #[test]
    fn all_three_buttons_show_when_the_system_allows_them() {
        assert_eq!(
            buttons(controls(true, true)),
            vec![Button::Minimize, Button::Maximize, Button::Close]
        );
    }

    #[test]
    fn buttons_the_system_does_not_offer_are_hidden_but_close_stays() {
        assert_eq!(buttons(controls(false, false)), vec![Button::Close]);
        assert_eq!(buttons(controls(false, true)), vec![Button::Maximize, Button::Close]);
    }

    #[test]
    fn maximize_turns_into_restore_when_the_window_is_maximized() {
        assert_eq!(Button::Maximize.glyph(false), "glyphs/window_maximize.svg");
        assert_eq!(Button::Maximize.glyph(true), "glyphs/window_restore.svg");
        assert_eq!(Button::Maximize.label(true), "Restaurar");
    }

    #[test]
    fn every_button_glyph_is_a_bundled_asset() {
        use gpui::AssetSource;
        for b in [Button::Minimize, Button::Maximize, Button::Close] {
            for maximized in [false, true] {
                let found = crate::app::assets::Assets.load(b.glyph(maximized)).unwrap();
                assert!(found.is_some(), "{}", b.glyph(maximized));
            }
        }
    }

    #[test]
    fn a_floating_window_resizes_from_every_edge_and_corner() {
        assert_eq!(resize_edges(Tiling::default(), false).len(), 8);
    }

    #[test]
    fn a_maximized_window_has_no_resize_edges() {
        assert!(resize_edges(Tiling::default(), true).is_empty());
    }

    #[test]
    fn a_window_tiled_to_the_left_keeps_only_the_untiled_edges() {
        let tiling = Tiling { top: true, left: true, right: false, bottom: true };
        assert_eq!(resize_edges(tiling, false), vec![ResizeEdge::Right]);
    }
}
