use std::collections::VecDeque;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::Term;
use alacritty_terminal::term::cell::Flags;

use crate::links::local_path;

pub(crate) fn viewport_point<T>(term: &Term<T>, col: usize, row: usize) -> Option<Point> {
    let rows = term.screen_lines();
    let cols = term.columns();
    if row >= rows || col >= cols {
        return None;
    }
    let offset = term.grid().display_offset() as i32;
    Some(Point::new(Line(row as i32 - offset), Column(col)))
}

fn wraps<T>(term: &Term<T>, line: Line) -> bool {
    let last = Column(term.columns() - 1);
    term.grid()[line][last].flags.contains(Flags::WRAPLINE)
}

fn push_row<T>(term: &Term<T>, line: Line, out: &mut Vec<char>) {
    let row = &term.grid()[line];
    for col in 0..term.columns() {
        let cell = &row[Column(col)];
        let spacer = cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER);
        out.push(if spacer || cell.c == '\0' { ' ' } else { cell.c });
    }
}

fn row_chars<T>(term: &Term<T>, line: Line) -> Vec<char> {
    let mut row = Vec::with_capacity(term.columns());
    push_row(term, line, &mut row);
    row
}

/// Claude Code cuts a word wider than the row at the last column, without WRAPLINE (2.1.325, 2026-11-16).
fn hard_break_indent(row: &[char], next: &[char]) -> Option<usize> {
    let blank = |c: &&char| **c == ' ';
    let tail = row.iter().rev().take_while(|c| !blank(c)).count();
    let indent = next.iter().take_while(blank).count();
    if tail == 0 || indent == 0 || indent >= next.len() {
        return None;
    }
    let head = next[indent..].iter().take_while(|c| !blank(c)).count();
    (tail + head > next.len() - indent).then_some(indent)
}

fn joint<T>(term: &Term<T>, line: Line, row: &[char], next: &[char]) -> Option<usize> {
    if wraps(term, line) { Some(0) } else { hard_break_indent(row, next) }
}

pub(crate) enum LinkText {
    Hyperlink(String),
    Plain(Vec<char>, usize),
}

pub(crate) fn hyperlink_at<T>(term: &Term<T>, point: Point) -> Option<String> {
    let cell = &term.grid()[point];
    let cell = if cell.flags.contains(Flags::WIDE_CHAR_SPACER) && point.column.0 > 0 {
        &term.grid()[Point::new(point.line, point.column - 1)]
    } else {
        cell
    };
    cell.hyperlink().map(|h| local_path(h.uri()))
}

pub(crate) fn link_text_at<T>(term: &Term<T>, col: usize, row: usize) -> Option<LinkText> {
    let point = viewport_point(term, col, row)?;
    if let Some(link) = hyperlink_at(term, point) {
        return Some(LinkText::Hyperlink(link));
    }
    let (top, bottom) = (term.grid().topmost_line(), term.grid().bottommost_line());
    let mut rows = VecDeque::from([row_chars(term, point.line)]);
    let mut skips = VecDeque::new();
    let mut first = point.line;
    while first > top {
        let prev = row_chars(term, first - 1);
        let Some(skip) = joint(term, first - 1, &prev, &rows[0]) else { break };
        rows.push_front(prev);
        skips.push_front(skip);
        first -= 1;
    }
    let mut last = point.line;
    while last < bottom {
        let next = row_chars(term, last + 1);
        let Some(skip) = joint(term, last, &rows[rows.len() - 1], &next) else { break };
        rows.push_back(next);
        skips.push_back(skip);
        last += 1;
    }
    let at = (point.line.0 - first.0) as usize;
    let mut text = Vec::with_capacity(rows.len() * term.columns());
    let mut column = None;
    for (i, row) in rows.iter().enumerate() {
        let skip = if i == 0 { 0 } else { skips[i - 1] };
        if i == at {
            column = point.column.0.checked_sub(skip).map(|c| text.len() + c);
        }
        text.extend_from_slice(&row[skip..]);
    }
    Some(LinkText::Plain(text, column?))
}

pub(crate) fn row_text<T>(term: &Term<T>, row: usize) -> String {
    let Some(point) = viewport_point(term, 0, row) else {
        return String::new();
    };
    let mut out = Vec::new();
    push_row(term, point.line, &mut out);
    out.into_iter().collect::<String>().trim_end().to_owned()
}
