use gpui::{FontWeight, IntoElement, ParentElement, Styled, div, px, relative};
use mx_core::layout::{Axis, DropSide, PaneNode};
use mx_store::PaneDrop;

use crate::app::theme::{Mx, RADIUS};

const CORE: f32 = 0.15;
const MAX_CORE: f32 = 160.;
pub const EDGE: f32 = 14.;

pub fn side_at(x: f32, y: f32, w: f32, h: f32) -> DropSide {
    let dx = x - w / 2.;
    let dy = y - h / 2.;
    let side = if dx.abs() < (w * CORE).min(MAX_CORE) && dy.abs() < (h * CORE).min(MAX_CORE) {
        DropSide::Center
    } else if (dx / w).abs() >= (dy / h).abs() {
        if dx < 0. { DropSide::Left } else { DropSide::Right }
    } else if dy < 0. {
        DropSide::Top
    } else {
        DropSide::Bottom
    };
    if side == DropSide::Center {
        return side;
    }
    let extent = match side.axis() {
        Axis::Row => w,
        Axis::Column => h,
    };
    if ((extent / 2.) as f64) < side.axis().min_extent() {
        DropSide::Center
    } else {
        side
    }
}

pub fn edge_at(x: f32, y: f32, w: f32, h: f32) -> Option<DropSide> {
    if x < 0. || y < 0. || x > w || y > h {
        return None;
    }
    [(x, DropSide::Left), (w - x, DropSide::Right), (y, DropSide::Top), (h - y, DropSide::Bottom)]
        .into_iter()
        .filter(|(d, _)| *d < EDGE)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, side)| side)
}

pub fn intent(side: DropSide, alt: bool, shift: bool, dragged_in_tree: bool) -> PaneDrop {
    match side {
        DropSide::Center if alt && dragged_in_tree => PaneDrop::Swap,
        DropSide::Center => PaneDrop::Replace,
        side if shift => PaneDrop::Edge(side),
        side => PaneDrop::Split(side),
    }
}

pub fn label(drop: PaneDrop) -> &'static str {
    match drop {
        PaneDrop::Replace => "Substituir",
        PaneDrop::Swap => "Trocar de lugar",
        PaneDrop::Split(_) => "Dividir",
        PaneDrop::Edge(_) => "Na borda da área",
    }
}

pub fn edge_share(tree: &PaneNode, side: DropSide) -> f32 {
    match tree.as_split() {
        Some(s) if s.axis == side.axis() => 1. / (s.children.len() as f32 + 1.),
        _ => 0.5,
    }
}

pub fn overlay(mx: &Mx, side: DropSide, share: f32, label: &'static str) -> impl IntoElement {
    let full = relative(1.);
    let zero = relative(0.);
    let part = relative(share);
    let rest = relative(1. - share);
    let (left, top, w, h) = match side {
        DropSide::Center => (zero, zero, full, full),
        DropSide::Left => (zero, zero, part, full),
        DropSide::Right => (rest, zero, part, full),
        DropSide::Top => (zero, zero, full, part),
        DropSide::Bottom => (zero, rest, full, part),
    };
    div()
        .absolute()
        .left(left)
        .top(top)
        .w(w)
        .h(h)
        .rounded(px(RADIUS))
        .bg(mx.accent.opacity(0.16))
        .border(px(1.6))
        .border_color(mx.accent.opacity(0.8))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .px(px(10.))
                .py(px(4.))
                .rounded(px(12.))
                .bg(mx.accent)
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(mx.canvas)
                .child(label),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_middle_of_a_pane_swaps() {
        assert_eq!(side_at(500., 300., 1000., 600.), DropSide::Center);
    }

    #[test]
    fn near_an_edge_the_pane_splits_on_that_side() {
        assert_eq!(side_at(20., 300., 1000., 600.), DropSide::Left);
        assert_eq!(side_at(980., 300., 1000., 600.), DropSide::Right);
        assert_eq!(side_at(500., 10., 1000., 600.), DropSide::Top);
        assert_eq!(side_at(500., 590., 1000., 600.), DropSide::Bottom);
    }

    #[test]
    fn the_core_is_capped_at_160_px() {
        assert_eq!(side_at(1000. + 170., 600., 2000., 1200.), DropSide::Right);
        assert_eq!(side_at(1000. + 150., 600., 2000., 1200.), DropSide::Center);
    }

    #[test]
    fn the_center_replaces_unless_alt_swaps_a_pane_already_shown() {
        assert_eq!(intent(DropSide::Center, false, false, true), PaneDrop::Replace);
        assert_eq!(intent(DropSide::Center, true, false, true), PaneDrop::Swap);
        assert_eq!(intent(DropSide::Center, true, false, false), PaneDrop::Replace);
        assert_eq!(intent(DropSide::Center, false, true, true), PaneDrop::Replace);
    }

    #[test]
    fn shift_over_a_side_goes_to_the_edge_of_the_area() {
        assert_eq!(intent(DropSide::Left, false, false, true), PaneDrop::Split(DropSide::Left));
        assert_eq!(intent(DropSide::Left, false, true, false), PaneDrop::Edge(DropSide::Left));
        assert_eq!(intent(DropSide::Bottom, true, true, true), PaneDrop::Edge(DropSide::Bottom));
        assert_eq!(intent(DropSide::Top, true, false, true), PaneDrop::Split(DropSide::Top));
    }

    #[test]
    fn the_pill_names_what_the_drop_does() {
        assert_eq!(label(PaneDrop::Replace), "Substituir");
        assert_eq!(label(PaneDrop::Swap), "Trocar de lugar");
        assert_eq!(label(PaneDrop::Split(DropSide::Right)), "Dividir");
        assert_eq!(label(PaneDrop::Edge(DropSide::Right)), "Na borda da área");
    }

    #[test]
    fn only_a_thin_strip_on_the_area_border_is_an_edge() {
        assert_eq!(edge_at(5., 300., 1000., 600.), Some(DropSide::Left));
        assert_eq!(edge_at(995., 300., 1000., 600.), Some(DropSide::Right));
        assert_eq!(edge_at(500., 3., 1000., 600.), Some(DropSide::Top));
        assert_eq!(edge_at(500., 597., 1000., 600.), Some(DropSide::Bottom));
        assert_eq!(edge_at(20., 300., 1000., 600.), None);
        assert_eq!(edge_at(-5., 300., 1000., 600.), None);
    }

    #[test]
    fn in_a_corner_the_nearer_edge_wins() {
        assert_eq!(edge_at(2., 10., 1000., 600.), Some(DropSide::Left));
        assert_eq!(edge_at(10., 2., 1000., 600.), Some(DropSide::Top));
    }

    #[test]
    fn the_edge_preview_matches_the_share_the_new_pane_gets() {
        let row = PaneNode::split(Axis::Row, vec![PaneNode::leaf("a"), PaneNode::leaf("b")], vec![0.5, 0.5]);
        assert!((edge_share(&row, DropSide::Right) - 1. / 3.).abs() < 1e-6);
        assert_eq!(edge_share(&row, DropSide::Top), 0.5);
        assert_eq!(edge_share(&PaneNode::leaf("a"), DropSide::Left), 0.5);
    }

    #[test]
    fn a_side_too_small_to_split_falls_back_to_center() {
        assert_eq!(side_at(10., 150., 400., 300.), DropSide::Center);
        assert_eq!(side_at(250., 5., 500., 250.), DropSide::Center);
    }
}
