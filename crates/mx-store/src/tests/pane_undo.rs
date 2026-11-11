use chrono::TimeDelta;
use mx_core::layout::{Axis, DropSide, PaneNode};

use super::Rig;
use crate::pane_undo::HISTORY_CAP;
use crate::{NOTHING_TO_UNDO, PaneDrop};

fn rig(ids: &[&str]) -> Rig {
    let mut rig = Rig::new();
    for id in ids {
        rig.panel(id);
    }
    rig
}

fn row(ids: &[&str]) -> PaneNode {
    let n = ids.len() as f64;
    PaneNode::split(Axis::Row, ids.iter().map(|id| PaneNode::leaf(*id)).collect(), vec![1. / n; ids.len()])
}

#[test]
fn a_replaced_pane_can_come_back() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.select("c");
    assert_eq!(rig.s.order(), ["a", "c"]);
    assert_eq!(rig.s.banner(), Some("b saiu da tela"));
    assert!(rig.s.can_undo_panes());
    assert!(rig.s.undo_panes());
    assert_eq!(rig.s.order(), ["a", "b"]);
    assert_eq!(rig.s.focused_pane_id(), Some("b"));
    assert_eq!(rig.s.banner(), None);
    assert!(!rig.s.can_undo_panes());
}

#[test]
fn a_dismissed_pane_can_come_back_with_its_weights() {
    let mut rig = rig(&["a", "b", "c"]);
    let tree = PaneNode::split(Axis::Row, vec![PaneNode::leaf("a"), PaneNode::leaf("b"), PaneNode::leaf("c")], vec![0.2, 0.5, 0.3]);
    rig.s.set_panes(Some(tree.clone()), Some("a"));
    rig.s.dismiss("b");
    assert_eq!(rig.s.banner(), Some("b saiu da tela"));
    rig.s.undo_panes();
    assert_eq!(rig.s.panes(), Some(&tree));
    assert_eq!(rig.s.focused_pane_id(), Some("a"));
}

#[test]
fn undoing_a_center_drop_gives_the_pin_back() {
    let mut rig = rig(&["fixo", "solto", "fora"]);
    rig.s.set_panes(Some(row(&["fixo", "solto"])), Some("solto"));
    rig.tm("fixo").pinned = true;
    rig.s.drop_tab("fora", "fixo", PaneDrop::Replace);
    assert!(rig.t("fora").pinned);
    assert_eq!(rig.s.banner(), Some("fixo saiu da tela"));
    rig.s.undo_panes();
    assert_eq!(rig.s.order(), ["fixo", "solto"]);
    assert!(rig.t("fixo").pinned);
    assert!(!rig.t("fora").pinned);
    assert_eq!(rig.s.focused_pane_id(), Some("solto"));
}

#[test]
fn splits_and_swaps_are_undone_without_a_banner() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("a"));
    rig.s.drop_tab("c", "a", PaneDrop::Split(DropSide::Bottom));
    let split = rig.s.panes().cloned();
    rig.s.drop_tab("a", "b", PaneDrop::Swap);
    assert_eq!(rig.s.banner(), None);
    assert!(rig.s.undo_panes());
    assert_eq!(rig.s.panes().cloned(), split);
    assert!(rig.s.undo_panes());
    assert_eq!(rig.s.panes(), Some(&row(&["a", "b"])));
}

#[test]
fn undos_walk_back_one_change_at_a_time() {
    let mut rig = rig(&["a", "b", "c", "d"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.select("c");
    rig.s.drop_tab("d", "a", PaneDrop::Split(DropSide::Right));
    rig.s.equalize_panes();
    assert!(rig.s.undo_panes());
    assert_eq!(rig.s.order(), ["a", "d", "c"]);
    assert_eq!(rig.s.panes().unwrap().as_split().unwrap().weights, [0.25, 0.25, 0.5]);
    assert!(rig.s.undo_panes());
    assert_eq!(rig.s.order(), ["a", "c"]);
    assert!(rig.s.undo_panes());
    assert_eq!(rig.s.order(), ["a", "b"]);
    assert!(!rig.s.undo_panes());
    assert_eq!(rig.s.banner(), Some(NOTHING_TO_UNDO));
}

#[test]
fn a_resize_is_undone_back_to_the_exact_weights() {
    let mut rig = rig(&["a", "b", "c"]);
    let tree = PaneNode::split(Axis::Row, vec![PaneNode::leaf("a"), PaneNode::leaf("b"), PaneNode::leaf("c")], vec![0.2, 0.5, 0.3]);
    rig.s.set_panes(Some(tree.clone()), Some("a"));
    rig.s.begin_resize();
    for _ in 0..5 {
        rig.s.resize_split(&[], 1, 30., 1000.);
    }
    assert_ne!(rig.s.panes(), Some(&tree));
    assert_eq!(rig.s.banner(), None);
    assert!(rig.s.undo_panes());
    assert_eq!(rig.s.panes(), Some(&tree));
    assert!(!rig.s.can_undo_panes());
}

#[test]
fn each_resize_gesture_is_its_own_step() {
    let mut rig = rig(&["a", "b"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("a"));
    rig.s.begin_resize();
    rig.s.resize_split(&[], 0, 100., 1000.);
    rig.s.resize_split(&[], 0, 100., 1000.);
    let first = rig.s.panes().cloned();
    rig.s.begin_resize();
    rig.s.resize_split(&[], 0, -50., 1000.);
    rig.s.undo_panes();
    assert_eq!(rig.s.panes().cloned(), first);
    rig.s.undo_panes();
    assert_eq!(rig.s.panes(), Some(&row(&["a", "b"])));
}

#[test]
fn a_gesture_that_moves_nothing_leaves_no_step() {
    let mut rig = rig(&["a", "b"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("a"));
    rig.s.begin_resize();
    rig.s.resize_split(&[], 0, 0., 1000.);
    assert!(!rig.s.can_undo_panes());
}

#[test]
fn the_history_keeps_only_the_latest_steps() {
    let mut rig = rig(&["a", "b"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("a"));
    for _ in 0..HISTORY_CAP + 5 {
        rig.s.begin_resize();
        rig.s.resize_split(&[], 0, 5., 1000.);
    }
    let mut undone = 0;
    while rig.s.undo_panes() {
        undone += 1;
    }
    assert_eq!(undone, HISTORY_CAP);
    let w = &rig.s.panes().unwrap().as_split().unwrap().weights;
    assert!((w[0] - 0.525).abs() < 1e-9);
}

#[test]
fn steps_with_a_closed_tab_are_skipped() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.select("c");
    rig.s.resize_split(&[], 0, 100., 1000.);
    rig.s.close_tab("c");
    assert!(rig.s.can_undo_panes());
    assert!(rig.s.undo_panes());
    assert_eq!(rig.s.panes(), Some(&row(&["a", "b"])));
}

#[test]
fn focus_and_zoom_are_not_steps() {
    let mut rig = rig(&["a", "b"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("a"));
    rig.s.focus_pane("b");
    rig.s.toggle_zoom();
    rig.s.toggle_zoom();
    assert!(!rig.s.can_undo_panes());
}

#[test]
fn the_banner_fades_but_the_step_stays() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.select("c");
    assert!(rig.s.banner_offers_undo());
    rig.advance(TimeDelta::seconds(7));
    assert_eq!(rig.s.banner(), None);
    assert!(!rig.s.banner_offers_undo());
    assert!(rig.s.undo_panes());
    assert_eq!(rig.s.order(), ["a", "b"]);
}

#[test]
fn another_banner_takes_the_button_but_not_the_step() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.select("c");
    rig.s.show_banner("outra coisa");
    assert!(!rig.s.banner_offers_undo());
    assert!(rig.s.undo_panes());
    assert_eq!(rig.s.banner(), Some("outra coisa"));
}

#[test]
fn closing_a_pane_is_not_a_step() {
    let mut rig = rig(&["a", "b"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.close_tab("b");
    assert!(!rig.s.can_undo_panes());
    assert_eq!(rig.s.banner(), None);
}

#[test]
fn an_undo_whose_pane_was_closed_since_does_nothing() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.select("c");
    rig.s.close_tab("b");
    assert!(!rig.s.undo_panes());
    assert_eq!(rig.s.order(), ["a", "c"]);
}

#[test]
fn cycling_sessions_is_not_a_step() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.cycle(1);
    assert_eq!(rig.s.order(), ["a", "c"]);
    assert_eq!(rig.s.banner(), None);
    assert!(!rig.s.can_undo_panes());
}

#[test]
fn a_sticky_banner_is_not_covered_but_the_step_is_kept() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.show_banner_sticky("importante");
    rig.s.select("c");
    assert_eq!(rig.s.banner(), Some("importante"));
    assert!(!rig.s.banner_offers_undo());
    assert!(rig.s.undo_panes());
    assert_eq!(rig.s.order(), ["a", "b"]);
    assert_eq!(rig.s.banner(), Some("importante"));
}
