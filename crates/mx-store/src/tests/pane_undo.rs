use chrono::TimeDelta;
use mx_core::layout::{Axis, DropSide, PaneNode};

use super::Rig;
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
fn splits_and_swaps_offer_nothing_to_undo() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("a"));
    rig.s.drop_tab("c", "a", PaneDrop::Split(DropSide::Bottom));
    rig.s.drop_tab("a", "b", PaneDrop::Swap);
    assert_eq!(rig.s.banner(), None);
    assert!(!rig.s.can_undo_panes());
}

#[test]
fn a_later_layout_change_spoils_the_undo() {
    let mut rig = rig(&["a", "b", "c", "d"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.select("c");
    rig.s.drop_tab("d", "a", PaneDrop::Split(DropSide::Right));
    assert!(!rig.s.can_undo_panes());
    let now = rig.s.panes().cloned();
    assert!(!rig.s.undo_panes());
    assert_eq!(rig.s.panes().cloned(), now);
    assert_eq!(rig.s.banner(), Some(NOTHING_TO_UNDO));
}

#[test]
fn focus_moves_keep_the_undo() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.select("c");
    rig.s.focus_pane("a");
    assert!(rig.s.undo_panes());
    assert_eq!(rig.s.order(), ["a", "b"]);
}

#[test]
fn the_undo_fades_with_its_banner() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.select("c");
    rig.advance(TimeDelta::seconds(7));
    assert_eq!(rig.s.banner(), None);
    assert!(!rig.s.can_undo_panes());
}

#[test]
fn another_banner_takes_the_undo_away() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.select("c");
    rig.s.show_banner("outra coisa");
        assert!(!rig.s.can_undo_panes());
}

#[test]
fn only_the_last_change_is_undone() {
    let mut rig = rig(&["a", "b", "c", "d"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.select("c");
    rig.s.select("d");
    assert_eq!(rig.s.banner(), Some("c saiu da tela"));
    rig.s.undo_panes();
    assert_eq!(rig.s.order(), ["a", "c"]);
    assert!(!rig.s.undo_panes());
}

#[test]
fn closing_a_pane_offers_no_undo() {
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
fn cycling_sessions_does_not_offer_undo() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.cycle(1);
    assert_eq!(rig.s.order(), ["a", "c"]);
    assert_eq!(rig.s.banner(), None);
}

#[test]
fn a_sticky_banner_is_not_covered_by_the_undo() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.show_banner_sticky("importante");
    rig.s.select("c");
    assert_eq!(rig.s.banner(), Some("importante"));
    assert!(!rig.s.can_undo_panes());
}
