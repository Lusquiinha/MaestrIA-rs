use gpui::{
    AppContext, Context, CursorStyle, Div, Entity, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement,
    Pixels, Point, Render, Stateful, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px, svg,
};

use crate::app::host::{StoreHost, update_store};

use crate::app::theme::Mx;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraggedTab {
    pub id: String,
    pub title: String,
    pub folder_root: String,
    pub from_tray: bool,
}

pub const GHOST_ANCHOR: (f32, f32) = (18., 17.);

pub fn under_cursor(grab: Point<Pixels>, ghost: impl IntoElement) -> Div {
    let (ax, ay) = GHOST_ANCHOR;
    // The drag view is laid out as a taffy root, which ignores its own insets.
    div().child(div().absolute().left(grab.x - px(ax)).top(grab.y - px(ay)).child(ghost))
}

pub struct TabGhost {
    pub title: String,
    pub at: Point<Pixels>,
}

impl Render for TabGhost {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mx = Mx::get(cx);
        under_cursor(
            self.at,
            div()
                .debug_selector(|| "tab-ghost".into())
                .w(px(240.))
                .px(px(12.))
                .py(px(9.))
                .rounded(px(12.))
                .bg(mx.bg_sidebar)
                .border_1()
                .border_color(mx.accent.opacity(0.55))
                .shadow(mx.card_shadow(18., 8.))
                .text_size(px(14.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(mx.fg)
                .truncate()
                .child(self.title.clone()),
        )
    }
}

pub struct HeaderGhost {
    pub title: String,
    pub tint: Option<Hsla>,
    pub at: Point<Pixels>,
}

impl Render for HeaderGhost {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mx = Mx::get(cx);
        under_cursor(
            self.at,
            div()
                .debug_selector(|| "header-ghost".into())
                .max_w(px(280.))
                .h(px(34.))
                .px(px(12.))
                .flex()
                .items_center()
                .gap(px(8.))
                .rounded(px(10.))
                .bg(mx.bg_sidebar)
                .border(px(1.4))
                .border_color(self.tint.unwrap_or(mx.accent.opacity(0.55)))
                .shadow(mx.card_shadow(18., 8.))
                .child(svg().path("glyphs/open_with.svg").flex_none().size(px(14.)).text_color(mx.fg_dim))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(mx.fg)
                        .child(self.title.clone()),
                ),
        )
    }
}

pub fn pane_grab(
    el: Stateful<Div>,
    tab: DraggedTab,
    tint: Option<Hsla>,
    host: Entity<StoreHost>,
    lifted: bool,
) -> Stateful<Div> {
    el.cursor(CursorStyle::OpenHand).when(lifted, |d| d.opacity(0.4)).on_drag(tab, move |d: &DraggedTab, at, _, cx| {
        update_store(&host, cx, |s| s.focus_pane(&d.id));
        cx.new(|_| HeaderGhost { title: d.title.clone(), tint, at })
    })
}
