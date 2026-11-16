use gpui::{Modifiers, MouseButton, NavigationDirection};
use mx_plugins::chord::{Chord, Key};
use serde_json::json;

use super::chord::{chord_id, check_chord_for, press, press_of_mouse};
use super::dispatch::{Focus, Hit, resolve};
use super::*;
use crate::chrome::settings::{ChordCheck, KeyPress as RecPress};

const BACK: MouseButton = MouseButton::Navigate(NavigationDirection::Back);
const FORWARD: MouseButton = MouseButton::Navigate(NavigationDirection::Forward);

fn none(_: &mx_plugins::chord::KeyPress) -> Option<String> {
    None
}

#[test]
fn the_side_buttons_are_mouse4_and_mouse5_and_the_others_are_not_shortcuts() {
    assert_eq!(press_of_mouse(BACK, Modifiers::none()).map(|p| p.key), Some(Key::Mouse(4)));
    assert_eq!(press_of_mouse(FORWARD, Modifiers::none()).map(|p| p.key), Some(Key::Mouse(5)));
    for b in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
        assert_eq!(press_of_mouse(b, Modifiers::none()), None);
        assert_eq!(recorded_mouse(b, Modifiers::none()), None);
    }
    let rec = recorded_mouse(BACK, Modifiers::control()).unwrap();
    assert_eq!((rec.key.as_str(), rec.ctrl), ("mouse4", true));
}

#[test]
fn the_recorder_accepts_a_side_button_without_modifier() {
    let p = RecPress { key: "mouse5".into(), ctrl: false, alt: false, shift: false, meta: false };
    assert_eq!(check_chord_for(&p, true), ChordCheck::Accepted { id: "mouse5".into(), label: "Mouse 5".into() });
    let left = RecPress { key: "mouse1".into(), ..p };
    assert!(matches!(check_chord_for(&left, true), ChordCheck::Rejected(_)));
}

#[test]
fn a_side_button_is_saved_by_its_id_and_loads_back() {
    let mut map = Keymap::defaults(true);
    map.bind(Action::PrevPane, Chord::new(Key::Mouse(4)), None);
    map.bind(Action::NextPane, Chord::new(Key::Mouse(5)), None);
    let saved = map.to_json();
    assert!(saved["prev-pane"].as_array().unwrap().contains(&json!("mouse4")));
    let loaded = Keymap::from_obj(&saved, true);
    assert_eq!(loaded.owner(&Chord::new(Key::Mouse(4))), Some(Action::PrevPane));
    assert_eq!(loaded.owner(&Chord::new(Key::Mouse(5))), Some(Action::NextPane));
    assert_eq!(chord_id(&Chord { control: true, ..Chord::new(Key::Mouse(5)) }), "ctrl+mouse5");
}

#[test]
fn a_saved_config_with_keys_only_loads_as_before() {
    let map = Keymap::load(&json!({"claude": ["meta+alt+t"]}), true);
    assert_eq!(map.chords(Action::Claude), [Chord::parse("meta+alt+t").unwrap()]);
    assert!(map.is_default(Action::NextPane));
}

#[test]
fn a_left_click_written_by_hand_in_the_config_is_dropped() {
    let map = Keymap::load(&json!({"prev-pane": ["mouse1", "mouse4"]}), true);
    assert_eq!(map.chords(Action::PrevPane), [Chord::new(Key::Mouse(4))]);
}

#[test]
fn a_side_button_fires_even_with_the_terminal_focused() {
    let mut map = Keymap::defaults(true);
    map.bind(Action::NextPane, Chord::new(Key::Mouse(5)), None);
    let p = press(Key::Mouse(5), false, false, false, false);
    let focus = Focus { terminal: true, ..Focus::default() };
    assert_eq!(resolve(&map, &p, focus, none, none), Some(Hit::Action(Action::NextPane)));
    let typing = Focus { text_field: true, ..Focus::default() };
    assert_eq!(resolve(&map, &p, typing, none, none), Some(Hit::Action(Action::NextPane)));
    let ctrl = press(Key::Mouse(5), true, false, false, false);
    assert_eq!(resolve(&map, &ctrl, focus, none, none), None);
}
