use mx_core::layout::{self, DropSide, PANE_GAP};
use mx_core::model::Folder;

use crate::changes::Areas;
use crate::placement::How;
use crate::store::Store;

impl Store {
    fn pane_toward(&self, side: DropSide) -> Option<(String, String)> {
        let from = self.focused_tab_id().filter(|f| self.is_open(f))?;
        let tree = self.panes.as_ref()?;
        let to = match self.pane_area {
            Some((w, h)) => {
                let rects = layout::rects(tree, w, h, PANE_GAP);
                layout::neighbour(&rects, &from, side)?.id.clone()
            }
            None => {
                let order = self.order();
                let at = order.iter().position(|t| *t == from)? as i64;
                let delta = if side.leading() { -1 } else { 1 };
                order[(at + delta).rem_euclid(order.len() as i64) as usize].clone()
            }
        };
        (to != from).then_some((from, to))
    }

    pub fn focus_toward(&mut self, side: DropSide) {
        if let Some((_, to)) = self.pane_toward(side) {
            self.focus_pane(&to);
        }
    }

    pub fn swap_toward(&mut self, side: DropSide) {
        let Some((from, to)) = self.pane_toward(side) else { return };
        if !self.panes.as_mut().is_some_and(|p| p.swap_leaves(&from, &to)) {
            return;
        }
        self.save();
        self.mark(Areas::PANES | Areas::SIDEBAR);
    }

    pub fn open_shell_toward(&mut self, folder: &Folder, feature: Option<&str>, side: DropSide) -> String {
        let id = self.open_shell_with(folder, None, None, feature, None, false);
        self.select_with(&id, How::Split(side));
        id
    }
}
