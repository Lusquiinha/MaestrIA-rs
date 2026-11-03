use gpui::{AnyElement, App, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, div, px};

use crate::app::host::update_store;
use crate::app::icons::{Icon, icon};
use crate::chrome::widgets::HOVER_RADIUS;
use crate::sidebar::tab_row::TabRow;
use crate::sidebar::tree::Ctx;
use crate::sidebar::widgets::count_text;

pub const EMPTY_TRAY: &str = "Quando você mandar um prompt numa sessão, ela aparece aqui.";

impl Ctx<'_> {
    pub fn active_tray(&self) -> Option<AnyElement> {
        let prefs = self.store.prefs();
        if !prefs.active_tray || self.store.filtering() {
            return None;
        }
        let mx = self.mx;
        let tabs = self.store.active_tray_tabs();
        let collapsed = prefs.active_tray_collapsed;
        let host = self.host.clone();
        let ruler = div()
            .id("active-tray-ruler")
            .ml(px(2.))
            .mr(px(7.))
            .pl(px(12.))
            .pr(px(7.))
            .mt(px(6.))
            .py(px(4.))
            .rounded(px(HOVER_RADIUS))
            .hover(|s| s.bg(mx.bg_hover))
            .flex()
            .items_center()
            .gap(px(6.))
            .cursor_pointer()
            .on_click(move |_, _, cx: &mut App| update_store(&host, cx, |s| s.toggle_active_tray_collapsed()))
            .child(icon(if collapsed { Icon::ChevronRight } else { Icon::ExpandMore }, 14., mx.fg_faint))
            .child(div().text_size(px(10.5)).text_color(mx.fg_faint).child("Em andamento"))
            .child(div().flex_1().h(px(1.)).bg(mx.border))
            .child(count_text(mx, tabs.len()));
        let mut col = div().flex().flex_col().child(ruler);
        if collapsed {
            return Some(col.into_any_element());
        }
        if tabs.is_empty() {
            col = col.child(
                div()
                    .px(px(20.))
                    .py(px(6.))
                    .text_size(px(12.))
                    .text_color(mx.fg_faint)
                    .child(EMPTY_TRAY),
            );
        }
        for t in tabs {
            col = col.child(TabRow::for_tray(self.store, mx, t).render(mx, self.host, None));
        }
        Some(col.into_any_element())
    }
}
