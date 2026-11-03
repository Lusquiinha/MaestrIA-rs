use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, FocusHandle, InteractiveElement, KeyContext, Subscription, Window};
use mx_plugins::chord::{Chord, KeyPress};
use mx_rfw::widgets::key_listener::KeyListener;
use mx_rfw::{Map, Value};

use super::layout::{Ctx, El, Fit, fire, owned, pass};
use super::widgets_core::optional;
use crate::keymap::{focus_from, press_of};

const CONTEXT: &str = "PluginKeys";
const KEYS: &str = "pluginKeys";
const PLUGIN_TEXT: &str = "PluginTextInput";

#[derive(Default)]
pub struct KeyListenerState {
    focus: Option<FocusHandle>,
    space: bool,
    blur: Option<Subscription>,
}

pub type KeySlot = Rc<RefCell<KeyListenerState>>;

impl KeyListenerState {
    pub fn release_space(&mut self) {
        self.space = false;
    }

    pub fn space_held(&self, window: &Window) -> bool {
        self.space && self.focus.as_ref().is_some_and(|h| h.is_focused(window))
    }
}

pub fn chords(keys: &[String]) -> Vec<(String, Chord)> {
    keys.iter().filter_map(|k| Some((k.clone(), Chord::parse(k)?))).collect()
}

pub fn matching(chords: &[(String, Chord)], p: &KeyPress) -> Option<String> {
    chords.iter().find(|(_, c)| c.accepts(p)).map(|(k, _)| k.clone())
}

fn context(keys: &[String]) -> KeyContext {
    let mut ctx = KeyContext::default();
    ctx.add(CONTEXT);
    ctx.set(KEYS, keys.join(" "));
    ctx
}

pub fn typing(stack: &[KeyContext]) -> bool {
    let f = focus_from(stack);
    f.terminal || f.text_field || stack.iter().any(|ctx| ctx.contains(PLUGIN_TEXT))
}

pub fn claimed(stack: &[KeyContext], p: &KeyPress) -> Option<String> {
    if typing(stack) {
        return None;
    }
    stack.iter().rev().filter_map(|ctx| ctx.get(KEYS).filter(|_| ctx.contains(CONTEXT))).find_map(|keys| {
        let declared: Vec<String> = keys.split(' ').filter(|k| !k.is_empty()).map(str::to_owned).collect();
        matching(&chords(&declared), p)
    })
}

pub fn render(cx: &mut Ctx, k: &KeyListener, fit: Fit) -> El {
    let slot = cx.widgets.filter(|_| cx.interactive).map(|states| {
        let slot = states.keys.next();
        let mut st = slot.borrow_mut();
        if st.focus.is_none() {
            st.focus = states.take_focus();
        }
        drop(st);
        slot
    });
    let handle = slot.as_ref().and_then(|s| s.borrow().focus.clone());
    let (Some(slot), Some(handle)) = (slot, handle) else {
        let inner = optional(cx, k.child, fit);
        return pass(cx.el(), inner);
    };
    let states = cx.widgets.expect("a key slot comes from the widget states");
    let outer = states.key_scope.replace(Some(slot.clone()));
    let inner = optional(cx, k.child, fit);
    states.key_scope.replace(outer);
    let (declared, events, sink) = (chords(&k.keys), owned(&k.on_key), cx.sink.clone());
    let (press_handle, down_slot, up_slot) = (handle.clone(), slot.clone(), slot);
    pass(cx.el(), inner)
        .key_context(context(&k.keys))
        .track_focus(&handle)
        .capture_any_mouse_down(move |_, window, app| {
            if !press_handle.contains_focused(window, app) {
                window.focus(&press_handle);
            }
        })
        .on_key_down(move |e, window, app| {
            if e.keystroke.key == "space" {
                remember_space(&down_slot, window, app);
            }
            let Some(p) = press_of(&e.keystroke) else { return };
            if typing(&window.context_stack()) {
                return;
            }
            let Some(key) = matching(&declared, &p) else { return };
            app.stop_propagation();
            fire(&sink, &events, &Map::from_iter([("key".to_owned(), Value::from(key))]), window, app);
        })
        .on_key_up(move |e, _, _| {
            if e.keystroke.key == "space" {
                up_slot.borrow_mut().space = false;
            }
        })
}

fn remember_space(slot: &KeySlot, window: &mut Window, app: &mut App) {
    let mut st = slot.borrow_mut();
    let Some(handle) = st.focus.clone().filter(|h| h.is_focused(window)) else { return };
    st.space = true;
    if st.blur.is_none() {
        let blur_slot = slot.clone();
        st.blur = Some(window.on_focus_out(&handle, app, move |_, _, _| blur_slot.borrow_mut().space = false));
    }
}

#[cfg(test)]
#[path = "key_listener_tests.rs"]
mod tests;
