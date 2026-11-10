use crate::layout::tree::{Axis, DropSide, MIN_HEIGHT, MIN_WIDTH, PaneNode};

pub const PANE_GAP: f64 = 8.0;

#[derive(Debug, Clone, PartialEq)]
pub struct PaneRect {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl PaneRect {
    pub fn area(&self) -> f64 {
        self.w * self.h
    }

    pub fn longer_axis(&self) -> Axis {
        if self.w >= self.h { Axis::Row } else { Axis::Column }
    }

    pub fn splits(&self, axis: Axis, gap: f64) -> bool {
        let (extent, min) = match axis {
            Axis::Row => (self.w, MIN_WIDTH),
            Axis::Column => (self.h, MIN_HEIGHT),
        };
        (extent - gap) / 2.0 >= min
    }
}

pub fn trailing(axis: Axis) -> DropSide {
    match axis {
        Axis::Row => DropSide::Right,
        Axis::Column => DropSide::Bottom,
    }
}

fn other(axis: Axis) -> Axis {
    match axis {
        Axis::Row => Axis::Column,
        Axis::Column => Axis::Row,
    }
}

pub fn rects(tree: &PaneNode, w: f64, h: f64, gap: f64) -> Vec<PaneRect> {
    let mut out = Vec::new();
    lay(tree, 0.0, 0.0, w.max(0.0), h.max(0.0), gap, &mut out);
    out
}

fn lay(node: &PaneNode, x: f64, y: f64, w: f64, h: f64, gap: f64, out: &mut Vec<PaneRect>) {
    let s = match node {
        PaneNode::Leaf(id) => {
            out.push(PaneRect { id: id.clone(), x, y, w, h });
            return;
        }
        PaneNode::Split(s) => s,
    };
    let n = s.children.len();
    if n == 0 {
        return;
    }
    let total = match s.axis {
        Axis::Row => w,
        Axis::Column => h,
    };
    let free = (total - gap * (n - 1) as f64).max(0.0);
    let sum: f64 = s.weights.iter().take(n).sum();
    let share = |i: usize| match s.weights.get(i) {
        Some(wt) if sum > 0.0 => wt / sum,
        _ => 1.0 / n as f64,
    };
    let mut at = 0.0;
    for (i, child) in s.children.iter().enumerate() {
        let size = free * share(i);
        match s.axis {
            Axis::Row => lay(child, x + at, y, size, h, gap, out),
            Axis::Column => lay(child, x, y + at, w, size, gap, out),
        }
        at += size + gap;
    }
}

pub fn rect_of(tree: &PaneNode, id: &str, w: f64, h: f64, gap: f64) -> Option<PaneRect> {
    rects(tree, w, h, gap).into_iter().find(|r| r.id == id)
}

fn split_of(r: &PaneRect, gap: f64) -> Option<DropSide> {
    let first = r.longer_axis();
    [first, other(first)].into_iter().find(|&a| r.splits(a, gap)).map(trailing)
}

pub fn auto_split(tree: &PaneNode, target: &str, w: f64, h: f64, gap: f64) -> Option<(String, DropSide)> {
    let all = rects(tree, w, h, gap);
    if let Some(side) = all.iter().find(|r| r.id == target).and_then(|r| split_of(r, gap)) {
        return Some((target.to_owned(), side));
    }
    let largest = biggest(all.iter().filter(|r| r.id != target))?;
    split_of(largest, gap).map(|side| (largest.id.clone(), side))
}

fn biggest<'a>(rects: impl Iterator<Item = &'a PaneRect>) -> Option<&'a PaneRect> {
    rects.reduce(|best, r| if r.area() > best.area() { r } else { best })
}

pub fn largest(tree: &PaneNode, w: f64, h: f64, gap: f64) -> Option<PaneRect> {
    biggest(rects(tree, w, h, gap).iter()).cloned()
}

pub fn neighbour<'a>(rects: &'a [PaneRect], from: &str, side: DropSide) -> Option<&'a PaneRect> {
    let r = rects.iter().find(|r| r.id == from)?;
    let span = |p: &PaneRect| match side.axis() {
        Axis::Row => (p.y, p.y + p.h),
        Axis::Column => (p.x, p.x + p.w),
    };
    let distance = |c: &PaneRect| match side {
        DropSide::Right => c.x - (r.x + r.w),
        DropSide::Left => r.x - (c.x + c.w),
        DropSide::Bottom => c.y - (r.y + r.h),
        DropSide::Top => r.y - (c.y + c.h),
        DropSide::Center => f64::NAN,
    };
    let (lo, hi) = span(r);
    let overlap = |c: &PaneRect| {
        let (a, b) = span(c);
        b.min(hi) - a.max(lo)
    };
    rects
        .iter()
        .filter(|c| c.id != from && distance(c) >= -0.5 && overlap(c) > 0.5)
        .min_by(|a, b| {
            let key = |c: &PaneRect| (distance(c), -overlap(c), span(c).0);
            key(a).partial_cmp(&key(b)).unwrap_or(std::cmp::Ordering::Equal)
        })
}
