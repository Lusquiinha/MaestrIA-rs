use mx_core::layout::{self, PaneNode};

use crate::changes::Areas;
use crate::store::Store;

pub const NOTHING_TO_UNDO: &str = "Nada pra desfazer nos painéis";

#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
    tree: Option<PaneNode>,
    focused: Option<String>,
    hole: Option<(PaneNode, String)>,
    pins: Vec<(String, bool)>,
}

#[derive(Debug, Clone)]
pub(crate) struct PaneUndo {
    before: Snapshot,
    after: Option<PaneNode>,
    screen: Option<String>,
}

#[derive(Debug, Default)]
pub(crate) struct Undo {
    last: Option<PaneUndo>,
    muted: bool,
}

impl Store {
    pub(crate) fn snapshot(&self) -> Snapshot {
        let pins = self.tabs.iter().map(|t| (t.id.clone(), t.pinned)).collect();
        Snapshot { tree: self.panes.clone(), focused: self.focused.clone(), hole: self.hole.clone(), pins }
    }

    pub(crate) fn offer_undo(&mut self, before: Snapshot, gone: &str) {
        if self.undo.muted || self.restoring || !self.has_tab(gone) || self.is_open(gone) {
            return;
        }
        let title = self.title_of(gone);
        let undo = PaneUndo { before, after: self.panes.clone(), screen: self.screen().map(str::to_owned) };
        self.show_banner(&format!("{title} saiu da tela"));
        self.undo.last = Some(undo);
    }

    pub(crate) fn muting_undo<T>(&mut self, f: impl FnOnce(&mut Store) -> T) -> T {
        let was = std::mem::replace(&mut self.undo.muted, true);
        let out = f(self);
        self.undo.muted = was;
        out
    }

    pub(crate) fn forget_undo(&mut self) {
        self.undo.last = None;
    }

    pub fn can_undo_panes(&self) -> bool {
        self.undo.last.as_ref().is_some_and(|u| self.undoable(u))
    }

    fn undoable(&self, u: &PaneUndo) -> bool {
        u.screen.as_deref() == self.screen()
            && u.after == self.panes
            && layout::order(u.before.tree.as_ref()).iter().all(|id| self.tab(id).is_some_and(|t| !t.embedded))
    }

    pub fn undo_panes(&mut self) -> bool {
        let Some(u) = self.undo.last.take().filter(|u| self.undoable(u)) else {
            self.show_banner(NOTHING_TO_UNDO);
            return false;
        };
        let Snapshot { tree, focused, hole, pins } = u.before;
        for (id, pinned) in pins {
            if let Some(t) = self.tab_mut(&id) {
                t.pinned = pinned;
            }
        }
        for id in layout::order(tree.as_ref()) {
            self.wake(&id);
        }
        self.panes = tree;
        self.focused = focused;
        self.hole = hole;
        self.clear_banner();
        self.save();
        self.mark(Areas::PANES | Areas::SIDEBAR);
        true
    }
}
