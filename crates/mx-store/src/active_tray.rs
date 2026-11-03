use mx_core::model::{Tab, TabKind};

use crate::changes::Areas;
use crate::store::Store;

impl Store {
    pub fn active_tray_tabs(&self) -> Vec<&Tab> {
        self.tabs.iter().filter(|t| t.in_active_tray).collect()
    }

    pub fn can_join_active_tray(&self, id: &str) -> bool {
        self.prefs.active_tray && self.tab(id).is_some_and(|t| t.kind == TabKind::Claude)
    }

    pub fn set_in_active_tray(&mut self, id: &str, on: bool) {
        let Some(t) = self.tab_mut(id) else { return };
        if t.in_active_tray == on {
            return;
        }
        t.in_active_tray = on;
        self.save();
        self.mark_tab(id);
        self.mark(Areas::SIDEBAR);
    }

    pub fn set_active_tray(&mut self, on: bool) {
        self.set_pref(on, |p| &mut p.active_tray);
        if on {
            let busy: Vec<String> = self
                .tabs
                .iter()
                .filter(|t| t.kind == TabKind::Claude && !t.in_active_tray && t.status().is_busy())
                .map(|t| t.id.clone())
                .collect();
            for id in busy {
                self.set_in_active_tray(&id, true);
            }
        }
        self.mark(Areas::SIDEBAR);
    }

    pub fn toggle_active_tray_collapsed(&mut self) {
        let c = !self.prefs.active_tray_collapsed;
        self.set_pref(c, |p| &mut p.active_tray_collapsed);
        self.mark(Areas::SIDEBAR);
    }
}
