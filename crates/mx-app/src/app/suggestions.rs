use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use mx_core::model::{SuggestedTask, SuggestionChoice};

use crate::app::{dialogs, ui};
use crate::chrome::dialog::frame::alert;
use crate::chrome::icon::{self, icon};
use crate::chrome::style::{MONO, cb, hsla, theme};
use crate::chrome::widgets::{Button, ButtonKind};

pub const WITHDRAWN: &str = "a sessão retirou essa sugestão enquanto o cartão estava aberto";

/// What the card's and the dialog's buttons do with a suggestion. With the tracker plugin's switch
/// on, every way out but dismissing creates the task there first, and the plugin answers back.
pub fn act(tab: &str, task: &str, choice: SuggestionChoice, window: &mut Window, cx: &mut App) {
    let store = ui::store(cx);
    if store.pending_suggestion(tab, task).is_none() {
        ui::act(cx, |s| s.show_banner(WITHDRAWN));
        return;
    }
    let via_plugin = store.suggestion_taker().is_some() && store.prefs().suggestions_to_plugin;
    if choice != SuggestionChoice::Dismiss && via_plugin {
        ui::act(cx, |s| s.start_suggestion_in_plugin(tab, task, choice));
        return;
    }
    match choice {
        SuggestionChoice::Dismiss => ui::act(cx, |s| s.dismiss_suggestion(tab, task)),
        SuggestionChoice::Here => {
            ui::act(cx, |s| s.run_suggestion_here(tab, task, None));
        }
        SuggestionChoice::Local => {
            ui::act(cx, |s| s.start_suggestion_here(tab, task, None, None));
        }
        SuggestionChoice::Worktree => dialogs::suggestion_task(tab, task, window, cx),
    }
}

/// The whole card in a dialog: title, summary, the prompt of the new session on demand, and the
/// ways out.
pub fn details(tab: &str, task: &str, cx: &mut App) {
    let store = ui::store(cx);
    let (Some(t), Some(s)) = (store.tab(tab), store.pending_suggestion(tab, task)) else { return };
    let dialog = SuggestionDialog {
        tab: tab.to_owned(),
        from: store.title(t),
        can_branch: !t.loose,
        task: s.clone(),
        prompt_open: false,
        generation: ui::upcoming(cx),
    };
    ui::modal(cx.new(|_| dialog).into(), cx);
}

struct SuggestionDialog {
    tab: String,
    from: String,
    can_branch: bool,
    task: SuggestedTask,
    prompt_open: bool,
    generation: u64,
}

impl SuggestionDialog {
    fn choose(&mut self, choice: SuggestionChoice, window: &mut Window, cx: &mut Context<Self>) {
        ui::close_if(self.generation, cx);
        act(&self.tab, &self.task.id, choice, window, cx);
    }
}

impl Render for SuggestionDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let me = cx.entity().downgrade();
        let task = &self.task;
        let title = if task.title.is_empty() { "tarefa sem título".to_owned() } else { task.title.clone() };
        let mut body = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .pb(px(10.))
                    .child(icon(icon::LIGHTBULB_OUTLINE_ROUNDED, 14., hsla(t.accent)))
                    .child(div().text_size(px(12.)).text_color(hsla(t.fg_dim)).child(format!("sugerida por {}", self.from))),
            )
            .child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).text_color(hsla(t.fg)).child(title));
        if !task.tldr.is_empty() {
            body = body.child(
                div().pt(px(8.)).text_size(px(13.)).line_height(px(18.8)).text_color(hsla(t.fg_dim)).child(task.tldr.clone()),
            );
        }
        let open = self.prompt_open;
        body = body.child(
            div()
                .id("suggestion-prompt-toggle")
                .mt(px(12.))
                .py(px(4.))
                .flex()
                .items_center()
                .gap(px(4.))
                .cursor_pointer()
                .on_click(move |_, w, cx| {
                    cb(me.clone(), |this: &mut SuggestionDialog, _, cx| {
                        this.prompt_open = !this.prompt_open;
                        cx.notify();
                    })(w, cx)
                })
                .child(icon(if open { icon::KEYBOARD_ARROW_UP } else { icon::KEYBOARD_ARROW_DOWN }, 14., hsla(t.fg_faint)))
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(hsla(t.fg_faint))
                        .child(if open { "esconder o prompt" } else { "ver o prompt da sessão nova" }),
                ),
        );
        if open {
            body = body.child(
                div()
                    .id("suggestion-prompt")
                    .mt(px(6.))
                    .max_h(px(320.))
                    .overflow_y_scroll()
                    .p(px(12.))
                    .rounded(px(6.))
                    .bg(hsla(t.bg))
                    .border_1()
                    .border_color(hsla(t.border))
                    .font_family(MONO)
                    .text_size(px(12.))
                    .line_height(px(17.4))
                    .text_color(hsla(t.fg))
                    .child(task.prompt.clone()),
            );
        }
        let me = cx.entity().downgrade();
        let go = |choice: SuggestionChoice| cb(me.clone(), move |this: &mut SuggestionDialog, w, cx| this.choose(choice, w, cx));
        let actions: Vec<AnyElement> = vec![
            Button::new("suggestion-dismiss", "descartar", ButtonKind::Text).on_click(go(SuggestionChoice::Dismiss)).into_any_element(),
            Button::new("suggestion-here", "fazer aqui", ButtonKind::Text).on_click(go(SuggestionChoice::Here)).into_any_element(),
            Button::new("suggestion-local", "iniciar localmente", ButtonKind::Text).on_click(go(SuggestionChoice::Local)).into_any_element(),
            Button::new("suggestion-worktree", "iniciar com worktree", ButtonKind::Primary)
                .disabled(!self.can_branch)
                .on_click(go(SuggestionChoice::Worktree))
                .into_any_element(),
        ];
        let generation = self.generation;
        alert("Tarefa sugerida", Some(560.), body, actions, Rc::new(move |_, cx| ui::close_if(generation, cx)), cx)
    }
}
