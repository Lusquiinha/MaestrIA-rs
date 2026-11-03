use std::rc::Rc;

use gpui::{App, AppContext, Context, IntoElement, ParentElement, Render, Styled, Window, div, px};
use mx_store::Store;

use crate::app::ui;
use crate::chrome::dialog::frame::alert;
use crate::chrome::icon::{self, Icon, icon};
use crate::chrome::style::{cb, hsla, theme};
use crate::chrome::widgets::{Button, ButtonKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    App,
    All,
    StaleDaemon { sessions: usize },
}

pub fn title(kind: Kind) -> &'static str {
    match kind {
        Kind::App => "Reiniciar o app?",
        Kind::All => "Reiniciar tudo?",
        Kind::StaleDaemon { .. } => "Atualizar o processo dos terminais?",
    }
}

fn ended(live: usize) -> String {
    match live {
        0 => "Não há sessão aberta pra encerrar".to_owned(),
        1 => "A sessão aberta é encerrada; o painel volta como estava, e o Claude volta com --resume".to_owned(),
        n => format!("As {n} sessões abertas são encerradas; os painéis voltam como estavam, e o Claude volta com --resume"),
    }
}

pub fn lines(kind: Kind, live: usize) -> Vec<(Icon, String)> {
    match kind {
        Kind::App => vec![
            (icon::RESTART_ALT_ROUNDED, "O app fecha e abre de novo".to_owned()),
            (icon::LOCK_OUTLINE_ROUNDED, ended(live)),
        ],
        Kind::All => vec![
            (icon::RESTART_ALT_ROUNDED, "O app e o processo que segura os terminais fecham e abrem de novo".to_owned()),
            (icon::LOCK_OUTLINE_ROUNDED, ended(live)),
        ],
        Kind::StaleDaemon { sessions } => vec![
            (
                icon::RESTART_ALT_ROUNDED,
                "Esta versão mudou o processo que segura os terminais, e o que está rodando ainda é o anterior".to_owned(),
            ),
            (icon::LOCK_OUTLINE_ROUNDED, ended(sessions)),
            (icon::SCHEDULE_ROUNDED, "Depois também serve: o anterior continua funcionando até lá".to_owned()),
        ],
    }
}

pub struct RestartConfirm {
    kind: Kind,
    lines: Vec<(Icon, String)>,
    generation: u64,
}

impl RestartConfirm {
    fn finish(&mut self, go: bool, cx: &mut Context<Self>) {
        ui::close_if(self.generation, cx);
        if go {
            restart(!matches!(self.kind, Kind::App), cx);
        }
    }
}

impl Render for RestartConfirm {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let me = cx.entity().downgrade();
        let body = div().flex().flex_col().children(self.lines.iter().map(|(glyph, text)| {
            div()
                .flex()
                .items_start()
                .gap(px(10.))
                .py(px(4.))
                .child(icon(*glyph, 16., hsla(t.fg_dim)))
                .child(div().flex_1().min_w_0().text_size(px(13.)).text_color(hsla(t.fg)).child(text.clone()))
        }));
        let actions = vec![
            Button::new("restart-later", "Depois", ButtonKind::Text)
                .on_click(cb(me.clone(), |this, _, cx| this.finish(false, cx)))
                .into_any_element(),
            Button::new("restart-now", "Reiniciar agora", ButtonKind::Primary)
                .on_click(cb(me.clone(), |this, _, cx| this.finish(true, cx)))
                .into_any_element(),
        ];
        alert(title(self.kind), Some(440.), body, actions, Rc::new(cb(me, |this, _, cx| this.finish(false, cx))), cx)
    }
}

fn live_sessions(cx: &App) -> usize {
    crate::status_bar::Summary::of(ui::store(cx)).sessions
}

pub fn confirm(kind: Kind, cx: &mut App) {
    let live = match kind {
        Kind::StaleDaemon { sessions } => sessions,
        _ => live_sessions(cx),
    };
    let lines = lines(kind, live);
    let generation = ui::upcoming(cx);
    let view = cx.new(|_| RestartConfirm { kind, lines, generation });
    ui::modal(view.into(), cx);
}

pub fn request(all: bool, cx: &mut App) {
    let kept = ui::host(cx).read(cx).terms.ptyd().is_some();
    match (all, kept) {
        (false, true) => restart(false, cx),
        (false, false) => confirm(Kind::App, cx),
        (true, _) => confirm(Kind::All, cx),
    }
}

pub fn app_exe() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(live_path(&exe.to_string_lossy()).into())
}

fn live_path(exe: &str) -> &str {
    exe.strip_suffix(" (deleted)").unwrap_or(exe)
}

fn restart(with_daemon: bool, cx: &mut App) {
    ui::act(cx, Store::keep_sessions_on_quit);
    if let Some(exe) = app_exe() {
        cx.set_restart_path(exe);
    }
    if with_daemon {
        ui::host(cx).update(cx, |h, _| h.stop_ptyd_on_quit = true);
    }
    cx.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_binary_replaced_by_an_update_restarts_from_its_path() {
        assert_eq!(live_path("/opt/maestria-rs/maestria-rs (deleted)"), "/opt/maestria-rs/maestria-rs");
        assert_eq!(live_path("/opt/maestria-rs/maestria-rs"), "/opt/maestria-rs/maestria-rs");
    }

    #[test]
    fn the_confirm_says_what_ends() {
        assert_eq!(title(Kind::All), "Reiniciar tudo?");
        assert_eq!(lines(Kind::All, 0)[1].1, "Não há sessão aberta pra encerrar");
        assert_eq!(
            lines(Kind::StaleDaemon { sessions: 3 }, 3)[1].1,
            "As 3 sessões abertas são encerradas; os painéis voltam como estavam, e o Claude volta com --resume"
        );
    }
}
