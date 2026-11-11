use std::collections::HashMap;

use mx_core::layout::{self, PaneNode};

use crate::changes::Areas;
use crate::store::Store;
use crate::timers::TimerKey;

pub const NOTHING_TO_UNDO: &str = "Nada pra desfazer nos painéis";
pub const HISTORY_CAP: usize = 30;

#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
    tree: Option<PaneNode>,
    focused: Option<String>,
    hole: Option<(PaneNode, String)>,
    pins: Vec<(String, bool)>,
}

#[derive(Debug, Clone)]
pub(crate) struct Mark {
    before: Snapshot,
    screen: Option<String>,
}

#[derive(Debug, Default)]
enum Gesture {
    #[default]
    Idle,
    Pending(Box<Mark>),
    Recorded,
}

#[derive(Debug, Default)]
pub(crate) struct Undo {
    history: HashMap<Option<String>, Vec<Snapshot>>,
    open: Option<Mark>,
    depth: usize,
    gone: Option<String>,
    gesture: Gesture,
    banner: bool,
    muted: bool,
}

impl Store {
    fn snapshot(&self) -> Snapshot {
        let pins = self.tabs.iter().map(|t| (t.id.clone(), t.pinned)).collect();
        Snapshot { tree: self.panes.clone(), focused: self.focused.clone(), hole: self.hole.clone(), pins }
    }

    fn layout_mark(&self) -> Mark {
        Mark { before: self.snapshot(), screen: self.screen().map(str::to_owned) }
    }

    pub(crate) fn tracked<T>(&mut self, f: impl FnOnce(&mut Store) -> T) -> T {
        let outer = self.undo.depth == 0;
        if outer {
            self.undo.open = Some(self.layout_mark());
            self.undo.gone = None;
        }
        self.undo.depth += 1;
        let out = f(self);
        self.undo.depth -= 1;
        if outer && let Some(mark) = self.undo.open.take() {
            let gone = self.undo.gone.take();
            if self.record(mark) && let Some(gone) = gone {
                self.offer_undo(&gone);
            }
        }
        out
    }

    pub(crate) fn rebase_tracking(&mut self) {
        if self.undo.open.is_some() {
            self.undo.open = Some(self.layout_mark());
        }
    }

    fn record(&mut self, mark: Mark) -> bool {
        let changed = mark.before.tree.is_some() && mark.before.tree != self.panes;
        if self.undo.muted || self.restoring || !changed || mark.screen.as_deref() != self.screen() {
            return false;
        }
        let stack = self.undo.history.entry(mark.screen).or_default();
        stack.push(mark.before);
        if stack.len() > HISTORY_CAP {
            stack.remove(0);
        }
        self.undo.gesture = Gesture::Idle;
        true
    }

    pub(crate) fn left_screen(&mut self, gone: &str) {
        if self.undo.depth > 0 {
            self.undo.gone = Some(gone.to_owned());
        }
    }

    fn offer_undo(&mut self, gone: &str) {
        let sticky = self.bell.banner.is_some() && !self.timers.is_set(&TimerKey::Banner);
        if sticky || !self.has_tab(gone) || self.is_open(gone) {
            return;
        }
        let title = self.title_of(gone);
        self.show_banner(&format!("{title} saiu da tela"));
        self.undo.banner = true;
    }

    pub fn begin_resize(&mut self) {
        self.undo.gesture = Gesture::Pending(Box::new(self.layout_mark()));
    }

    pub(crate) fn resizing<T>(&mut self, f: impl FnOnce(&mut Store) -> T) -> T {
        let mark = match std::mem::take(&mut self.undo.gesture) {
            Gesture::Pending(mark) => Some((*mark, true)),
            Gesture::Recorded => None,
            Gesture::Idle => Some((self.layout_mark(), false)),
        };
        let out = f(self);
        match mark {
            None => self.undo.gesture = Gesture::Recorded,
            Some((mark, gesture)) => {
                let kept = gesture.then(|| mark.clone());
                let recorded = self.record(mark);
                self.undo.gesture = match (recorded, kept) {
                    (true, Some(_)) => Gesture::Recorded,
                    (false, Some(mark)) => Gesture::Pending(Box::new(mark)),
                    (_, None) => Gesture::Idle,
                };
            }
        }
        out
    }

    pub(crate) fn muting_undo<T>(&mut self, f: impl FnOnce(&mut Store) -> T) -> T {
        let was = std::mem::replace(&mut self.undo.muted, true);
        let out = f(self);
        self.undo.muted = was;
        out
    }

    pub(crate) fn forget_undo_banner(&mut self) {
        self.undo.banner = false;
    }

    pub(crate) fn forget_layout_history(&mut self) {
        let screen = self.screen().map(str::to_owned);
        self.undo.history.remove(&screen);
        self.undo.gesture = Gesture::Idle;
    }

    pub fn banner_offers_undo(&self) -> bool {
        self.undo.banner && self.can_undo_panes()
    }

    pub fn can_undo_panes(&self) -> bool {
        self.undo
            .history
            .get(&self.screen().map(str::to_owned))
            .is_some_and(|stack| stack.iter().any(|s| self.restorable(s)))
    }

    fn restorable(&self, s: &Snapshot) -> bool {
        layout::order(s.tree.as_ref()).iter().all(|id| self.tab(id).is_some_and(|t| !t.embedded))
    }

    pub fn undo_panes(&mut self) -> bool {
        let screen = self.screen().map(str::to_owned);
        let mut stack = self.undo.history.remove(&screen).unwrap_or_default();
        let mut found = None;
        while let Some(s) = stack.pop() {
            if self.restorable(&s) {
                found = Some(s);
                break;
            }
        }
        if !stack.is_empty() {
            self.undo.history.insert(screen, stack);
        }
        self.undo.gesture = Gesture::Idle;
        let Some(Snapshot { tree, focused, hole, pins }) = found else {
            self.show_banner(NOTHING_TO_UNDO);
            return false;
        };
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
        self.settle_zoom();
        if self.undo.banner {
            self.clear_banner();
        }
        self.save();
        self.mark(Areas::PANES | Areas::SIDEBAR);
        true
    }
}
