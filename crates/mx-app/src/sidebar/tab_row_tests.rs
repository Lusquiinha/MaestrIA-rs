use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    Context, Entity, InteractiveElement, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};
use mx_core::model::Folder;
use mx_store::testing::{FakeBus, FakeClock, FakeHooks, FakeNotifier, FakeTerms};
use mx_store::{Ports, Store, StoreOptions};
use mx_sys::FakeRunner;

use super::TabRow;
use crate::app::drag::DraggedTab;
use crate::app::host::StoreHost;
use crate::app::theme::Mx;

struct Tray {
    host: Entity<StoreHost>,
    dropped: Rc<RefCell<Vec<String>>>,
}

impl Render for Tray {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mx = Mx::get(cx).clone();
        let store = &self.host.read(cx).store;
        let rows: Vec<_> = store
            .active_tray_tabs()
            .into_iter()
            .map(|t| div().debug_selector(|| "tray-row".into()).child(TabRow::for_tray(store, &mx, t).render(&mx, &self.host, None)))
            .collect();
        let dropped = self.dropped.clone();
        div()
            .size_full()
            .flex()
            .child(div().w(px(260.)).flex().flex_col().children(rows))
            .child(
                div()
                    .id("pane")
                    .debug_selector(|| "pane".into())
                    .flex_1()
                    .h(px(400.))
                    .on_drop(move |d: &DraggedTab, _, _| dropped.borrow_mut().push(d.id.clone())),
            )
    }
}

struct Rig {
    host: Entity<StoreHost>,
    dropped: Rc<RefCell<Vec<String>>>,
    id: String,
    dir: std::path::PathBuf,
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn mount<'a>(cx: &'a mut TestAppContext, name: &str) -> (Rig, &'a mut VisualTestContext) {
    let rt = Arc::new(tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap());
    let dir = std::env::temp_dir().join(format!("mx-tray-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ports = Ports {
        clock: Arc::new(FakeClock::new(chrono::Local::now())),
        terms: Arc::new(FakeTerms::default()),
        runner: Arc::new(FakeRunner::new()),
        notifier: Arc::new(FakeNotifier::default()),
        plugins: Arc::new(FakeBus::default()),
        hooks: Arc::new(FakeHooks { port: 4242 }),
        rt: rt.handle().clone(),
    };
    let mut store = Store::new(StoreOptions::at(dir.join("state"), dir.join("chats"), dir.to_str()), ports);
    let id = store.open_shell(&Folder::new("/repo", "repo"), None, None, None, None);
    store.dismiss(&id);
    store.set_in_active_tray(&id, true);
    let host = cx.update(|cx| {
        Mx::install(mx_theme::default_theme(), cx);
        StoreHost::for_test(store, rt, cx)
    });
    let dropped: Rc<RefCell<Vec<String>>> = Rc::default();
    let (h, d) = (host.clone(), dropped.clone());
    let (_, vcx) = cx.add_window_view(move |_, _| Tray { host: h, dropped: d });
    vcx.run_until_parked();
    (Rig { host, dropped, id, dir }, vcx)
}

#[gpui::test]
fn a_session_dragged_from_the_tray_lands_on_the_pane_and_stays_in_the_tray(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx, "drag");
    let from = cx.debug_bounds("tray-row").expect("tray row drawn").center();
    let to = cx.debug_bounds("pane").expect("pane drawn").center();
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(*rig.dropped.borrow(), vec![rig.id.clone()]);
    let in_tray = rig.host.read_with(cx, |h, _| h.store.tab(&rig.id).is_some_and(|t| t.in_active_tray));
    assert!(in_tray);
}

#[gpui::test]
fn clicking_a_tray_row_still_opens_the_session(cx: &mut TestAppContext) {
    let (rig, cx) = mount(cx, "click");
    assert!(!rig.host.read_with(cx, |h, _| h.store.is_open(&rig.id)));
    let at = cx.debug_bounds("tray-row").expect("tray row drawn").center();
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert!(rig.host.read_with(cx, |h, _| h.store.is_open(&rig.id)));
    assert!(rig.dropped.borrow().is_empty());
}
