use mx_core::model::sidebar::RowKey;

use crate::changes::Areas;
use crate::store::Store;
use crate::workspaces::RowPlace;

/// Where a dragged sidebar row would land if dropped now: next to `onto`
/// (on the side it ends up, `below` or above) or inside that workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropHint {
    pub onto: RowPlace,
    pub into: bool,
    pub below: bool,
}

fn slide(list: &mut Vec<String>, from: Option<usize>, to: Option<usize>) -> bool {
    let (Some(from), Some(to)) = (from, to) else {
        return false;
    };
    if from == to || from >= list.len() || to >= list.len() {
        return false;
    }
    let item = list.remove(from);
    list.insert(to, item);
    true
}

impl Store {
    pub fn move_root_row(&mut self, row: &RowKey, target: &RowKey) {
        self.pin_root_order();
        let idx = |k: &RowKey| self.root_order.iter().position(|x| *x == k.key());
        let (from, to) = (idx(row), idx(target));
        if slide(&mut self.root_order, from, to) {
            self.save();
            self.mark(Areas::SIDEBAR);
        }
    }

    pub fn move_in_workspace(&mut self, ws: &str, root: &str, target: &str) {
        let Some(w) = self.workspaces.iter_mut().find(|w| w.id == ws) else {
            return;
        };
        let idx = |r: &str| w.folder_roots.iter().position(|x| x == r);
        let (from, to) = (idx(root), idx(target));
        if slide(&mut w.folder_roots, from, to) {
            self.save();
            self.mark(Areas::SIDEBAR);
        }
    }

    pub fn move_folder(&mut self, root: &str, from: Option<&str>, to: &str, before: Option<&str>) {
        self.add_to_workspace(root, to, before);
        if let Some(from) = from.filter(|f| *f != to) {
            self.remove_from_workspace(root, from, None);
        }
    }

    pub fn can_drop(&self, from: &RowPlace, onto: &RowPlace, into: bool) -> bool {
        if into {
            return matches!(from.row, RowKey::Folder(_))
                && matches!(&onto.row, RowKey::Workspace(w) if from.within.as_deref() != Some(w.as_str()));
        }
        if from.row == onto.row && from.within == onto.within {
            return false;
        }
        if matches!(from.row, RowKey::Workspace(_)) {
            return onto.within.is_none();
        }
        true
    }

    pub fn drop_row(&mut self, from: &RowPlace, onto: &RowPlace, into: bool) {
        if !self.can_drop(from, onto, into) {
            return;
        }
        let RowKey::Folder(root) = &from.row else {
            self.move_root_row(&from.row, &onto.row);
            return;
        };
        if into {
            if let RowKey::Workspace(to) = &onto.row {
                self.move_folder(root, from.within.as_deref(), to, None);
            }
            return;
        }
        match (&onto.within, &from.within) {
            (None, None) => self.move_root_row(&from.row, &onto.row),
            (None, Some(w)) => self.remove_from_workspace(root, w, Some(&onto.row)),
            (Some(lane), within) => {
                let RowKey::Folder(target) = &onto.row else { return };
                if within.as_deref() == Some(lane.as_str()) {
                    self.move_in_workspace(lane, root, target);
                } else {
                    self.move_folder(root, within.as_deref(), lane, Some(target));
                }
            }
        }
    }

    /// The hint for `from` hovering `onto` at `y_frac` (0 top, 1 bottom) of
    /// its header. Only a folder goes into a workspace, and only through the
    /// middle half of the header (`accepts_into`); the edges reorder.
    pub fn drop_hint(&self, from: &RowPlace, onto: &RowPlace, accepts_into: bool, y_frac: f32) -> Option<DropHint> {
        if from == onto {
            return None;
        }
        let into = accepts_into && matches!(from.row, RowKey::Folder(_)) && y_frac > 0.25 && y_frac < 0.75;
        self.can_drop(from, onto, into).then(|| DropHint {
            onto: onto.clone(),
            into,
            below: !into && self.lands_below(from, onto),
        })
    }

    /// Sliding within the same lane puts a row coming from above *after* the
    /// target. From another lane it takes the target's slot, above it.
    fn lands_below(&self, from: &RowPlace, onto: &RowPlace) -> bool {
        if from.within != onto.within {
            return false;
        }
        let order: Vec<String> = match &onto.within {
            None => self.sidebar_rows().iter().map(RowKey::key).collect(),
            Some(ws) => self.folders_of(ws).iter().map(|f| RowKey::Folder(f.root.clone()).key()).collect(),
        };
        let at = |p: &RowPlace| order.iter().position(|k| *k == p.row.key());
        matches!((at(from), at(onto)), (Some(a), Some(b)) if a < b)
    }

    pub fn has_hits_in_workspace(&self, ws: &str) -> bool {
        self.members_of(ws).iter().any(|f| self.has_hits(&f.root))
    }
}
