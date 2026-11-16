use gpui::{
    App, AppContext, Bounds, DragMoveEvent, InteractiveElement, ParentElement, Pixels, Point, Stateful,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};
use mx_core::model::sidebar::RowKey;
use mx_store::RowPlace;

use crate::app::drag::{DraggedRow, TabGhost};
use crate::app::host::update_store;
use crate::sidebar::tree::Ctx;

fn y_frac(bounds: &Bounds<Pixels>, p: &Point<Pixels>) -> f32 {
    let h = f32::from(bounds.size.height);
    if h <= 0. { 0.5 } else { f32::from(p.y - bounds.origin.y) / h }
}

impl Ctx<'_> {
    /// Makes a folder or workspace header draggable and a drop target for
    /// other headers, with the stripe / "entra aqui" highlight on top.
    pub fn row_drag(&self, el: Stateful<gpui::Div>, place: RowPlace, title: String, accepts_into: bool) -> Stateful<gpui::Div> {
        let mx = self.mx;
        let hint = self.row_hint.filter(|h| h.onto == place);
        let lifted = self.lifted.is_some_and(|l| *l == place);
        let drag = DraggedRow { place: place.clone(), title };
        let sidebar = self.sidebar.clone();
        let host = self.host.clone();
        let mine = place.clone();
        let drop_sidebar = self.sidebar.clone();
        let drop_host = self.host.clone();
        let selector = format!("row:{}@{}", place.row.key(), place.within.as_deref().unwrap_or(""));
        let drop_onto = place;
        el.relative()
            .debug_selector(move || selector)
            .when(lifted, |d| d.opacity(0.35))
            .on_drag(drag, |d: &DraggedRow, at, _, cx| cx.new(|_| TabGhost { title: d.title.clone(), at }))
            .on_drag_move(move |ev: &DragMoveEvent<DraggedRow>, _, cx: &mut App| {
                let from = ev.drag(cx).place.clone();
                let p = ev.event.position;
                let next = ev
                    .bounds
                    .contains(&p)
                    .then(|| host.read(cx).store.drop_hint(&from, &mine, accepts_into, y_frac(&ev.bounds, &p)));
                let _ = sidebar.update(cx, |s, cx| {
                    let mut changed = s.lifted.as_ref() != Some(&from);
                    s.lifted = Some(from);
                    // Outside this header: only clear a hint that points at it,
                    // never one another header just set.
                    let ours = s.row_hint.as_ref().is_some_and(|h| h.onto == mine);
                    if let Some(next) = next.or(ours.then_some(None))
                        && next != s.row_hint
                    {
                        s.row_hint = next;
                        changed = true;
                    }
                    if changed {
                        cx.notify();
                    }
                });
            })
            .on_drop(move |d: &DraggedRow, _, cx: &mut App| {
                let hint = drop_sidebar
                    .update(cx, |s, cx| {
                        s.lifted = None;
                        cx.notify();
                        s.row_hint.take()
                    })
                    .ok()
                    .flatten()
                    .filter(|h| h.onto == drop_onto);
                if let Some(h) = hint {
                    let from = d.place.clone();
                    update_store(&drop_host, cx, |s| s.drop_row(&from, &h.onto, h.into));
                }
            })
            .when_some(hint, |d, h| {
                if h.into {
                    d.child(
                        div()
                            .absolute()
                            .inset_0()
                            .rounded(px(6.))
                            .bg(mx.accent.opacity(0.12))
                            .border(px(1.5))
                            .border_color(mx.accent),
                    )
                } else {
                    d.child(
                        div()
                            .absolute()
                            .left(px(7.))
                            .right(px(7.))
                            .h(px(2.5))
                            .rounded(px(2.))
                            .bg(mx.accent)
                            .map(|s| if h.below { s.bottom_0() } else { s.top_0() }),
                    )
                }
            })
    }

    /// The "Arraste uma pasta pra cá" line of an empty workspace takes a
    /// folder in, like the middle of the header.
    pub fn empty_workspace_drop(&self, el: gpui::Div, ws: &str) -> Stateful<gpui::Div> {
        let onto = RowPlace::root(RowKey::Workspace(ws.to_owned()));
        let host = self.host.clone();
        let check = onto.clone();
        let accent = self.mx.accent;
        el.id(gpui::SharedString::from(format!("ws-empty:{ws}")))
            .rounded(px(6.))
            .border_1()
            .border_color(gpui::transparent_black())
            .drag_over::<DraggedRow>(move |s, d, _, cx| {
                if host.read(cx).store.can_drop(&d.place, &check, true) {
                    s.bg(accent.opacity(0.12)).border_color(accent)
                } else {
                    s
                }
            })
            .on_drop({
                let host = self.host.clone();
                move |d: &DraggedRow, _, cx: &mut App| {
                    let from = d.place.clone();
                    update_store(&host, cx, |s| s.drop_row(&from, &onto, true))
                }
            })
    }
}

#[cfg(test)]
#[path = "row_drag_tests.rs"]
mod tests;
