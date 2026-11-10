use mx_core::layout::{Axis, DropSide, PaneNode};
use mx_core::model::Folder;

use super::Rig;

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
fn directional_focus_follows_the_geometry() {
    let mut rig = rig(&["a", "b", "c"]);
    let right = PaneNode::split(Axis::Column, vec![PaneNode::leaf("b"), PaneNode::leaf("c")], vec![0.5, 0.5]);
    rig.s.set_panes(Some(PaneNode::split(Axis::Row, vec![PaneNode::leaf("a"), right], vec![0.5, 0.5])), Some("c"));
    rig.s.set_pane_area(1008., 608.);
    rig.s.focus_toward(DropSide::Top);
    assert_eq!(rig.s.focused_pane_id(), Some("b"));
    rig.s.focus_toward(DropSide::Left);
    assert_eq!(rig.s.focused_pane_id(), Some("a"));
    rig.s.focus_toward(DropSide::Left);
    assert_eq!(rig.s.focused_pane_id(), Some("a"));
}

#[test]
fn directional_focus_without_a_measured_area_walks_the_order() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b", "c"])), Some("a"));
    rig.s.focus_toward(DropSide::Bottom);
    assert_eq!(rig.s.focused_pane_id(), Some("b"));
    rig.s.focus_toward(DropSide::Left);
    rig.s.focus_toward(DropSide::Left);
    assert_eq!(rig.s.focused_pane_id(), Some("c"));
}

#[test]
fn swapping_toward_a_neighbour_keeps_the_focus_on_the_moved_pane() {
    let mut rig = rig(&["a", "b", "c"]);
    rig.s.set_panes(Some(row(&["a", "b", "c"])), Some("a"));
    rig.s.set_pane_area(1016., 600.);
    rig.s.swap_toward(DropSide::Right);
    assert_eq!(rig.s.order(), ["b", "a", "c"]);
    assert_eq!(rig.s.focused_pane_id(), Some("a"));
    rig.s.swap_toward(DropSide::Top);
    assert_eq!(rig.s.order(), ["b", "a", "c"]);
}

#[test]
fn a_new_shell_below_splits_the_focused_pane() {
    let mut rig = rig(&["a", "b"]);
    rig.s.set_panes(Some(row(&["a", "b"])), Some("b"));
    rig.s.set_pane_area(1008., 600.);
    let id = rig.s.open_shell_toward(&Folder::new("/repo", "repo"), None, DropSide::Bottom);
    let below = PaneNode::split(Axis::Column, vec![PaneNode::leaf("b"), PaneNode::leaf(id.clone())], vec![0.5, 0.5]);
    assert_eq!(rig.s.panes(), Some(&PaneNode::split(Axis::Row, vec![PaneNode::leaf("a"), below], vec![0.5, 0.5])));
    assert_eq!(rig.s.focused_pane_id(), Some(id.as_str()));
}
