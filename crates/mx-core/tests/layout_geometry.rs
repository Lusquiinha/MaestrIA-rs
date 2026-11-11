use mx_core::layout::{self, Axis, DropSide, PANE_GAP, PaneNode, PaneRect};

fn leaf(id: &str) -> PaneNode {
    PaneNode::leaf(id)
}

fn rect(id: &str, x: f64, y: f64, w: f64, h: f64) -> PaneRect {
    PaneRect { id: id.into(), x, y, w, h }
}

#[test]
fn a_single_leaf_fills_the_whole_area() {
    assert_eq!(layout::rects(&leaf("a"), 1000., 600., PANE_GAP), vec![rect("a", 0., 0., 1000., 600.)]);
}

#[test]
fn a_row_shares_the_width_left_after_the_gaps_by_weight() {
    let tree = PaneNode::split(Axis::Row, vec![leaf("a"), leaf("b")], vec![0.25, 0.75]);
    assert_eq!(
        layout::rects(&tree, 1008., 600., PANE_GAP),
        vec![rect("a", 0., 0., 250., 600.), rect("b", 258., 0., 750., 600.)]
    );
}

#[test]
fn nested_splits_place_every_leaf_where_the_flex_layout_puts_it() {
    let right = PaneNode::split(Axis::Column, vec![leaf("b"), leaf("c")], vec![0.5, 0.5]);
    let tree = PaneNode::split(Axis::Row, vec![leaf("a"), right], vec![0.5, 0.5]);
    assert_eq!(
        layout::rects(&tree, 1008., 608., PANE_GAP),
        vec![
            rect("a", 0., 0., 500., 608.),
            rect("b", 508., 0., 500., 300.),
            rect("c", 508., 308., 500., 300.),
        ]
    );
}

#[test]
fn three_children_lose_two_gaps() {
    let tree = PaneNode::split(Axis::Column, vec![leaf("a"), leaf("b"), leaf("c")], vec![1. / 3., 1. / 3., 1. / 3.]);
    let r = layout::rects(&tree, 400., 316., PANE_GAP);
    assert!(r.iter().all(|r| (r.h - 100.).abs() < 1e-9));
    assert!((r[2].y - 216.).abs() < 1e-9);
}

#[test]
fn a_wide_pane_splits_to_the_right() {
    assert_eq!(layout::auto_split(&leaf("a"), "a", 1200., 600., PANE_GAP), Some(("a".into(), DropSide::Right)));
}

#[test]
fn a_tall_pane_splits_below() {
    assert_eq!(layout::auto_split(&leaf("a"), "a", 600., 1000., PANE_GAP), Some(("a".into(), DropSide::Bottom)));
}

#[test]
fn a_wide_pane_too_narrow_for_two_columns_splits_below() {
    assert_eq!(layout::auto_split(&leaf("a"), "a", 450., 400., PANE_GAP), Some(("a".into(), DropSide::Bottom)));
}

#[test]
fn a_cramped_target_hands_the_split_to_the_largest_pane() {
    let tree = PaneNode::split(Axis::Row, vec![leaf("a"), leaf("b")], vec![0.2, 0.8]);
    assert_eq!(layout::auto_split(&tree, "a", 1208., 250., PANE_GAP), Some(("b".into(), DropSide::Right)));
}

#[test]
fn no_room_anywhere_means_no_split() {
    let tree = PaneNode::split(Axis::Row, vec![leaf("a"), leaf("b")], vec![0.5, 0.5]);
    assert_eq!(layout::auto_split(&tree, "a", 600., 250., PANE_GAP), None);
}

#[test]
fn halves_must_keep_the_minimums() {
    let r = rect("a", 0., 0., 2. * layout::MIN_WIDTH + PANE_GAP, 2. * layout::MIN_HEIGHT + PANE_GAP - 1.);
    assert!(r.splits(Axis::Row, PANE_GAP));
    assert!(!r.splits(Axis::Column, PANE_GAP));
}

#[test]
fn the_largest_pane_is_the_first_of_equals() {
    let tree = PaneNode::split(Axis::Row, vec![leaf("a"), leaf("b"), leaf("c")], vec![0.25, 0.5, 0.25]);
    assert_eq!(layout::largest(&tree, 1000., 500., PANE_GAP).map(|r| r.id), Some("b".into()));
    let even = PaneNode::split(Axis::Row, vec![leaf("a"), leaf("b")], vec![0.5, 0.5]);
    assert_eq!(layout::largest(&even, 1000., 500., PANE_GAP).map(|r| r.id), Some("a".into()));
}

fn grid() -> Vec<PaneRect> {
    let right = PaneNode::split(Axis::Column, vec![leaf("b"), leaf("c")], vec![0.5, 0.5]);
    let tree = PaneNode::split(Axis::Row, vec![leaf("a"), right], vec![0.5, 0.5]);
    layout::rects(&tree, 1008., 608., PANE_GAP)
}

fn next(rects: &[PaneRect], from: &str, side: DropSide) -> Option<String> {
    layout::neighbour(rects, from, side).map(|r| r.id.clone())
}

#[test]
fn the_neighbour_is_the_pane_touching_that_side() {
    let g = grid();
    assert_eq!(next(&g, "b", DropSide::Bottom).as_deref(), Some("c"));
    assert_eq!(next(&g, "c", DropSide::Top).as_deref(), Some("b"));
    assert_eq!(next(&g, "b", DropSide::Left).as_deref(), Some("a"));
    assert_eq!(next(&g, "c", DropSide::Left).as_deref(), Some("a"));
}

#[test]
fn a_tall_pane_goes_to_the_topmost_of_the_panes_beside_it() {
    assert_eq!(next(&grid(), "a", DropSide::Right).as_deref(), Some("b"));
}

#[test]
fn the_widest_overlap_wins_over_a_corner_touch() {
    let rects = vec![
        rect("a", 0., 0., 500., 300.),
        rect("b", 508., 0., 500., 100.),
        rect("c", 508., 108., 500., 192.),
    ];
    assert_eq!(next(&rects, "a", DropSide::Right).as_deref(), Some("c"));
}

#[test]
fn there_is_no_neighbour_past_the_edge_or_for_an_unknown_pane() {
    let g = grid();
    assert_eq!(next(&g, "a", DropSide::Left), None);
    assert_eq!(next(&g, "b", DropSide::Top), None);
    assert_eq!(next(&g, "a", DropSide::Bottom), None);
    assert_eq!(next(&g, "z", DropSide::Right), None);
}

#[test]
fn a_pane_further_away_loses_to_the_adjacent_one() {
    let tree = PaneNode::split(Axis::Row, vec![leaf("a"), leaf("b"), leaf("c")], vec![1., 1., 1.]);
    let rects = layout::rects(&tree, 1016., 600., PANE_GAP);
    assert_eq!(next(&rects, "a", DropSide::Right).as_deref(), Some("b"));
    assert_eq!(next(&rects, "c", DropSide::Left).as_deref(), Some("b"));
}

fn grip(path: &[usize], gutter: usize) -> layout::GripRef {
    layout::GripRef { path: path.to_vec(), gutter }
}

fn half(axis: Axis, a: PaneNode, b: PaneNode) -> PaneNode {
    PaneNode::split(axis, vec![a, b], vec![0.5, 0.5])
}

#[test]
fn a_single_split_has_no_junction() {
    let tree = half(Axis::Row, leaf("a"), leaf("b"));
    assert!(layout::junctions(&tree, 1008., 600., PANE_GAP).is_empty());
    assert!(layout::junctions(&leaf("a"), 1008., 600., PANE_GAP).is_empty());
}

#[test]
fn a_t_junction_joins_the_parent_and_the_child_grip() {
    let tree = half(Axis::Row, leaf("a"), half(Axis::Column, leaf("b"), leaf("c")));
    let j = layout::junctions(&tree, 1008., 608., PANE_GAP);
    assert_eq!(j.len(), 1);
    assert_eq!((j[0].x, j[0].y), (504., 304.));
    assert_eq!(j[0].rows, [grip(&[], 0)]);
    assert_eq!(j[0].columns, [grip(&[1], 0)]);
}

#[test]
fn an_aligned_two_by_two_grid_meets_in_one_cross() {
    let tree = half(Axis::Row, half(Axis::Column, leaf("a"), leaf("b")), half(Axis::Column, leaf("c"), leaf("d")));
    let j = layout::junctions(&tree, 1008., 608., PANE_GAP);
    assert_eq!(j.len(), 1);
    assert_eq!((j[0].x, j[0].y), (504., 304.));
    assert_eq!(j[0].rows, [grip(&[], 0)]);
    assert_eq!(j[0].columns, [grip(&[0], 0), grip(&[1], 0)]);
}

#[test]
fn an_offset_two_by_two_grid_has_two_t_junctions() {
    let col = |a, b, top: f64| PaneNode::split(Axis::Column, vec![leaf(a), leaf(b)], vec![top, 1. - top]);
    let tree = half(Axis::Row, col("a", "b", 0.25), col("c", "d", 0.75));
    let j = layout::junctions(&tree, 1008., 608., PANE_GAP);
    assert_eq!(j.len(), 2);
    assert_eq!((j[0].x, j[0].y, j[0].columns.clone()), (504., 154., vec![grip(&[0], 0)]));
    assert_eq!((j[1].x, j[1].y, j[1].columns.clone()), (504., 454., vec![grip(&[1], 0)]));
}

#[test]
fn the_area_of_a_nested_split_leaves_out_the_gap() {
    let tree = half(Axis::Row, leaf("a"), half(Axis::Column, leaf("b"), leaf("c")));
    assert_eq!(layout::split_area(&tree, &[1], 1008., 608., PANE_GAP), Some(layout::Area { x: 508., y: 0., w: 500., h: 608. }));
    assert_eq!(layout::split_area(&tree, &[0], 1008., 608., PANE_GAP), None);
}
