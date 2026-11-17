//! Redesenho de animações contínuas sem pedir quadro a quadro.
//!
//! `window.request_animation_frame()` avisa a view inteira a cada quadro (120 Hz
//! numa tela ProMotion) e, no gpui, avisar uma view suja também todas as de
//! cima dela. Um spinner de "carregando" numa tela de plugin remontava a tela, o
//! `Panes` e o `Root` 120 vezes por segundo. Aqui a view é avisada no máximo a
//! cada `FRAME` (ou `IDLE` com a janela em segundo plano), uma vez só por view,
//! mesmo com vários spinners nela.

use std::cell::RefCell;
use std::collections::HashSet;
use std::time::Duration;

use gpui::{App, EntityId, Global, Window};

use crate::chrome::notices::anim_off;

/// ~30 quadros por segundo: liso pra um spinner, um quarto do custo a 120 Hz.
pub const FRAME: Duration = Duration::from_millis(33);
/// Janela em segundo plano: a animação segue, devagar.
pub const IDLE: Duration = Duration::from_millis(250);

#[derive(Default)]
struct Pending(RefCell<HashSet<EntityId>>);

impl Global for Pending {}

/// Pede um redesenho da view que está sendo pintada daqui a pouco. Chame no
/// `paint` de quem anima, no lugar de `window.request_animation_frame()`.
pub fn redraw_soon(window: &mut Window, cx: &mut App) {
    if anim_off() {
        return;
    }
    let view = window.current_view();
    if !cx.default_global::<Pending>().0.borrow_mut().insert(view) {
        return;
    }
    let wait = if window.is_window_active() { FRAME } else { IDLE };
    window
        .spawn(cx, async move |cx| {
            cx.background_executor().timer(wait).await;
            cx.update(|_, cx| {
                cx.default_global::<Pending>().0.borrow_mut().remove(&view);
                cx.notify(view);
            })
            .ok();
        })
        .detach();
}
