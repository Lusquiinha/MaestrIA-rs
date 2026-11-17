//! `MX_PERF_TOUR=1` (junto com `MX_PERF=1`): depois de abrir, passeia sozinho
//! pelas telas que têm trabalho de primeira vez (configurações, catálogo de
//! plugins e os primeiros plugins do rail), escrevendo `perf tour <passo>` no
//! log antes de cada uma. Serve para comparar antes e depois sem clicar.

use std::time::Duration;

use gpui::{AnyWindowHandle, App, AppContext, AsyncApp};

use crate::app::{settings, ui};
use crate::chrome::settings::Section;

const STEP: Duration = Duration::from_secs(3);

pub fn requested() -> bool {
    crate::perf::enabled() && std::env::var_os("MX_PERF_TOUR").is_some_and(|v| !v.is_empty() && v != "0")
}

pub fn start(window: AnyWindowHandle, cx: &mut App) {
    cx.spawn(async move |cx: &mut AsyncApp| {
        let wait = async |cx: &mut AsyncApp| cx.background_executor().timer(STEP).await;
        wait(cx).await;
        for section in Section::ALL {
            let label = format!("settings {section:?}");
            step(cx, window, &label, move |w, cx| settings::open(section, w, cx));
            wait(cx).await;
        }
        step(cx, window, "catalog", |w, cx| {
            ui::close_settings(cx);
            crate::plugin_ui::open_catalog(w, cx);
        });
        wait(cx).await;
        let plugins = cx.update(|cx| ui::store(cx).rail_plugins()).unwrap_or_default();
        step(cx, window, "catalog.close", |_, cx| crate::plugin_ui::close_dialog(cx));
        for id in plugins {
            wait(cx).await;
            let label = format!("plugin {id}");
            step(cx, window, &label, move |_, cx| ui::act(cx, |s| s.show_sidebar_view(Some(&id))));
        }
        wait(cx).await;
        step(cx, window, "sessions", |_, cx| ui::act(cx, |s| s.show_sidebar_view(None)));
        wait(cx).await;
        log::info!("perf tour done");
    })
    .detach();
}

fn step(
    cx: &mut AsyncApp,
    window: AnyWindowHandle,
    label: &str,
    f: impl FnOnce(&mut gpui::Window, &mut App),
) {
    log::info!("perf tour {label}");
    if let Err(err) = cx.update_window(window, |_, w, cx| f(w, cx)) {
        log::warn!("perf tour {label}: {err:#}");
    }
}
