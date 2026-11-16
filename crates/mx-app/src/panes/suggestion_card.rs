//! The tasks a session suggested (`suggest_task`), in a card floating over the bottom corner of its
//! terminal: the newest in front, the others one click away. Folded, it becomes a badge in the same
//! corner; a new suggestion unfolds it. It only exists while there is one pending.

use gpui::{
    AnyElement, App, Context, Div, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, SharedString,
    Stateful, StatefulInteractiveElement, Styled, WeakEntity, Window, div, prelude::FluentBuilder, px,
};
use mx_core::model::{SuggestionChoice, Tab, TabKind};

use crate::app::suggestions;
use crate::app::theme::Mx;
use crate::chrome::icon::{self, Icon, icon};
use crate::chrome::style::{alpha, on, shadow};
use crate::chrome::widgets::{IconButton, MxSwitch, tip};
use crate::panes::PaneArea;

pub const INSET: f32 = 12.;
pub const WIDTH: f32 = 340.;

#[derive(Default)]
pub struct CardState {
    index: usize,
    folded: bool,
    more: bool,
    known: Vec<String>,
    shown: Option<String>,
}

impl CardState {
    /// `ids` newest first. A suggestion not seen before unfolds the card and comes to the front.
    pub fn sync(&mut self, ids: &[String]) {
        if ids.iter().any(|id| !self.known.contains(id)) {
            self.folded = false;
            self.index = 0;
        }
        self.known = ids.to_vec();
        self.index = self.index.min(ids.len().saturating_sub(1));
        let current = ids.get(self.index).cloned();
        if current != self.shown {
            self.more = false;
            self.shown = current;
        }
    }
}

type Edit = fn(&mut CardState, usize);

fn edit(me: &WeakEntity<PaneArea>, tab: &str, count: usize, f: Edit) -> impl Fn(&mut Window, &mut App) + 'static {
    let (me, tab) = (me.clone(), tab.to_owned());
    move |_, cx| {
        me.update(cx, |this, cx| {
            if let Some(c) = this.cards.get_mut(&tab) {
                f(c, count);
                cx.notify();
            }
        })
        .ok();
    }
}

fn choose(tab: &str, task: &str, choice: SuggestionChoice) -> impl Fn(&mut Window, &mut App) + 'static {
    let (tab, task) = (tab.to_owned(), task.to_owned());
    move |w, cx| suggestions::act(&tab, &task, choice, w, cx)
}

fn compact(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
    let label = label.into();
    let selector = label.clone();
    div()
        .id(id)
        .debug_selector(move || format!("button:{selector}"))
        .flex()
        .items_center()
        .justify_center()
        .h(px(28.))
        .px(px(12.))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .whitespace_nowrap()
        .cursor_pointer()
        .child(label)
}

fn start_option(id: &'static str, glyph: Icon, label: &'static str, detail: &'static str, mx: &Mx) -> Stateful<Div> {
    let hover = mx.bg_hover;
    div()
        .id(id)
        .debug_selector(move || format!("option:{label}"))
        .mt(px(2.))
        .px(px(8.))
        .py(px(7.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .gap(px(10.))
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(icon(glyph, 14., mx.fg_faint))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(div().text_size(px(12.5)).font_weight(FontWeight::MEDIUM).text_color(mx.fg).truncate().child(label))
                .child(div().text_size(px(11.)).text_color(mx.fg_faint).truncate().child(detail)),
        )
}

impl PaneArea {
    pub(super) fn sync_cards(&mut self, cx: &App) {
        let store = &self.host.read(cx).store;
        let pending: Vec<(String, Vec<String>)> = store
            .tabs()
            .iter()
            .filter(|t| t.kind == TabKind::Claude && !t.hooks.suggestions.is_empty())
            .map(|t| (t.id.clone(), t.hooks.suggestions.iter().rev().map(|s| s.id.clone()).collect()))
            .collect();
        self.cards.retain(|id, _| pending.iter().any(|(p, _)| p == id));
        for (id, ids) in pending {
            self.cards.entry(id).or_default().sync(&ids);
        }
    }

    pub(super) fn suggestion_card(&self, tab: &Tab, mx: &Mx, cx: &Context<Self>) -> Option<AnyElement> {
        let state = self.cards.get(&tab.id)?;
        let pending: Vec<_> = tab.hooks.suggestions.iter().rev().collect();
        let task = *pending.get(state.index)?;
        let count = pending.len();
        let me = cx.weak_entity();
        let inner = if state.folded {
            let text = if count == 1 { "1 sugestão".to_owned() } else { format!("{count} sugestões") };
            let tooltip = if count == 1 { "mostrar a tarefa sugerida".to_owned() } else { format!("mostrar as {count} tarefas sugeridas") };
            let hover = mx.bg_hover;
            div()
                .id("suggestion-badge")
                .debug_selector(|| "suggestion-badge".into())
                .occlude()
                .h(px(28.))
                .px(px(10.))
                .flex()
                .items_center()
                .gap(px(6.))
                .rounded(px(14.))
                .bg(mx.bg_sidebar)
                .border_1()
                .border_color(mx.accent.opacity(0.45))
                .shadow(shadow(&mx.theme, 10., 3.))
                .cursor_pointer()
                .hover(move |s| s.bg(hover))
                .tooltip(tip(tooltip))
                .on_click({
                    let unfold = edit(&me, &tab.id, count, |c, _| c.folded = false);
                    move |_, w, cx| unfold(w, cx)
                })
                .child(icon(icon::LIGHTBULB_OUTLINE_ROUNDED, 13., mx.accent))
                .child(div().text_size(px(11.5)).text_color(mx.fg_dim).child(text))
                .into_any_element()
        } else {
            self.card(tab, task, state, count, &me, mx, cx)
        };
        Some(
            div()
                .absolute()
                .left(px(INSET))
                .right(px(INSET))
                .bottom(px(INSET))
                .flex()
                .justify_end()
                .child(inner)
                .into_any_element(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn card(
        &self,
        tab: &Tab,
        task: &mx_core::model::SuggestedTask,
        state: &CardState,
        count: usize,
        me: &WeakEntity<PaneArea>,
        mx: &Mx,
        cx: &Context<Self>,
    ) -> AnyElement {
        let store = &self.host.read(cx).store;
        let can_branch = !tab.loose;
        let taker = store.suggestion_taker_label();
        let via_plugin = taker.is_some() && store.prefs().suggestions_to_plugin;
        let taker_name = taker.as_ref().map(|(name, _)| name.clone());
        let id = &tab.id;

        let mut head = div()
            .flex()
            .items_center()
            .gap(px(2.))
            .child(icon(icon::LIGHTBULB_OUTLINE_ROUNDED, 13., mx.accent))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .pl(px(4.))
                    .text_size(px(11.))
                    .text_color(mx.fg_faint)
                    .truncate()
                    .child("tarefa sugerida"),
            );
        if count > 1 {
            head = head
                .child(div().text_size(px(11.)).text_color(mx.fg_faint).child(format!("{}/{count}", state.index + 1)))
                .child(
                    IconButton::new("suggestion-next", icon::CHEVRON_RIGHT)
                        .size(22., 15., 6.)
                        .tooltip("a próxima")
                        .on_click(edit(me, id, count, |c, n| c.index = (c.index + 1) % n)),
                );
        }
        head = head
            .child(
                IconButton::new("suggestion-fold", icon::REMOVE)
                    .size(22., 15., 6.)
                    .tooltip("recolher")
                    .on_click(edit(me, id, count, |c, _| c.folded = true)),
            )
            .child(
                IconButton::new("suggestion-dismiss", icon::CLOSE_ROUNDED)
                    .size(22., 15., 6.)
                    .tooltip("descartar")
                    .on_click(choose(id, &task.id, SuggestionChoice::Dismiss)),
            );

        let title = if task.title.is_empty() { "tarefa sem título".to_owned() } else { task.title.clone() };
        let mut text = div()
            .pt(px(8.))
            .pr(px(6.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(
                div()
                    .text_size(px(13.5))
                    .line_height(px(17.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(mx.fg)
                    .line_clamp(2)
                    .text_ellipsis()
                    .child(title),
            );
        if !task.tldr.is_empty() {
            text = text.child(
                div()
                    .text_size(px(12.))
                    .line_height(px(17.4))
                    .text_color(mx.fg_dim)
                    .line_clamp(3)
                    .text_ellipsis()
                    .child(task.tldr.clone()),
            );
        }

        let more = state.more.then(|| {
            let local = start_option("suggestion-local", icon::TERMINAL, "iniciar localmente", "sessão nova nesta pasta, sem worktree", mx)
                .on_click({
                    let go = choose(id, &task.id, SuggestionChoice::Local);
                    move |_, w, cx| go(w, cx)
                });
            let here = start_option("suggestion-here", icon::SUBDIRECTORY_ARROW_RIGHT, "fazer aqui", "manda o prompt pra esta sessão", mx)
                .on_click({
                    let go = choose(id, &task.id, SuggestionChoice::Here);
                    move |_, w, cx| go(w, cx)
                });
            div()
                .pt(px(10.))
                .pr(px(6.))
                .flex()
                .flex_col()
                .child(div().h(px(1.)).mb(px(4.)).bg(mx.border))
                .when(can_branch, |d| d.child(local))
                .child(here)
        });

        let switch = taker.map(|(_, label)| {
            let on_now = store.prefs().suggestions_to_plugin;
            let host = self.host.clone();
            let toggle = move |value: bool, _: &mut Window, cx: &mut App| {
                crate::app::host::update_store(&host, cx, |s| s.set_suggestions_to_plugin(value));
            };
            let row_toggle = toggle.clone();
            div()
                .pt(px(10.))
                .pr(px(2.))
                .flex()
                .justify_end()
                .child(
                    div()
                        .id("suggestion-plugin")
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .min_w_0()
                        .cursor_pointer()
                        .on_click(move |_, w, cx| row_toggle(!on_now, w, cx))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(px(11.5))
                                .text_color(if on_now { mx.fg } else { mx.fg_dim })
                                .child(label),
                        )
                        .child(MxSwitch::new("suggestion-plugin-switch", on_now).on_change(toggle)),
                )
        });

        let start_tip = match (taker_name, via_plugin, can_branch) {
            (Some(name), true, true) => format!("cria a tarefa no {name}, a worktree dela e abre a sessão"),
            (Some(name), true, false) => format!("cria a tarefa no {name} e abre a sessão nesta pasta"),
            (_, _, true) => "pede o id da task, cria a worktree e abre a sessão nela".to_owned(),
            (_, _, false) => "sessão nova nesta mesma pasta".to_owned(),
        };
        let (fill, fill_fg) = (mx.accent, on(mx.theme.accent, mx.theme.canvas));
        let fill_hover = alpha(mx.theme.accent, 0.88);
        let start_choice = if can_branch { SuggestionChoice::Worktree } else { SuggestionChoice::Local };
        let start = compact("suggestion-start", if can_branch { "iniciar com worktree" } else { "iniciar localmente" })
            .min_w_0()
            .rounded_l(px(6.))
            .bg(fill)
            .text_color(fill_fg)
            .hover(move |s| s.bg(fill_hover))
            .tooltip(tip(start_tip))
            .on_click({
                let go = choose(id, &task.id, start_choice);
                move |_, w, cx| go(w, cx)
            });
        let arrow = div()
            .id("suggestion-more")
            .debug_selector(|| "suggestion-more".into())
            .flex()
            .items_center()
            .justify_center()
            .h(px(28.))
            .px(px(6.))
            .rounded_r(px(6.))
            .bg(fill)
            .cursor_pointer()
            .hover(move |s| s.bg(fill_hover))
            .tooltip(tip(if state.more { "esconder as outras formas" } else { "outras formas de iniciar" }))
            .on_click({
                let toggle = edit(me, id, count, |c, _| c.more = !c.more);
                move |_, w, cx| toggle(w, cx)
            })
            .child(icon(if state.more { icon::KEYBOARD_ARROW_DOWN } else { icon::KEYBOARD_ARROW_UP }, 16., fill_fg));
        let details = compact("suggestion-details", "detalhes")
            .rounded(px(6.))
            .text_color(mx.fg_dim)
            .hover({
                let hover = mx.bg_hover;
                move |s| s.bg(hover)
            })
            .tooltip(tip("o cartão inteiro, com o prompt da sessão nova"))
            .on_click({
                let (tab, task) = (id.clone(), task.id.clone());
                move |_, _, cx| suggestions::details(&tab, &task, cx)
            });
        let buttons = div()
            .pt(px(if switch.is_some() { 6. } else { 12. }))
            .pr(px(6.))
            .flex()
            .flex_wrap()
            .justify_end()
            .gap(px(4.))
            .child(details)
            .child(div().flex().min_w_0().gap(px(1.)).child(start).child(arrow));

        div()
            .id("suggestion-card")
            .debug_selector(|| "suggestion-card".into())
            .occlude()
            .w_full()
            .max_w(px(WIDTH))
            .flex()
            .flex_col()
            .pt(px(10.))
            .pb(px(12.))
            .pl(px(14.))
            .pr(px(8.))
            .rounded(px(10.))
            .bg(mx.bg_sidebar)
            .border_1()
            .border_color(mx.border)
            .shadow(shadow(&mx.theme, 18., 6.))
            .child(head)
            .child(text)
            .children(more)
            .children(switch)
            .child(buttons)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::CardState;

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_new_suggestion_unfolds_the_card_and_comes_to_the_front() {
        let mut c = CardState::default();
        c.sync(&ids(&["b", "a"]));
        c.index = 1;
        c.folded = true;
        c.sync(&ids(&["b", "a"]));
        assert!(c.folded);
        assert_eq!(c.index, 1);
        c.sync(&ids(&["c", "b"]));
        assert!(!c.folded);
        assert_eq!(c.index, 0);
    }

    #[test]
    fn the_options_close_when_the_shown_suggestion_changes() {
        let mut c = CardState::default();
        c.sync(&ids(&["b", "a"]));
        c.more = true;
        c.sync(&ids(&["b", "a"]));
        assert!(c.more);
        c.sync(&ids(&["a"]));
        assert!(!c.more);
        assert_eq!(c.index, 0);
    }
}
