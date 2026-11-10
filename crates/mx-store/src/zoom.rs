use crate::changes::Areas;
use crate::store::Store;

impl Store {
    pub fn zoomed(&self) -> Option<&str> {
        self.zoomed.as_deref()
    }

    pub fn toggle_zoom(&mut self) {
        match self.focused_tab_id() {
            Some(id) => self.toggle_zoom_of(&id),
            None => {
                self.unzoom();
            }
        }
    }

    pub fn toggle_zoom_of(&mut self, id: &str) {
        if self.zoomed.as_deref() == Some(id) {
            self.unzoom();
            return;
        }
        if !self.is_open(id) || self.pane_count() < 2 {
            return;
        }
        self.focused = Some(id.to_owned());
        self.zoomed = Some(id.to_owned());
        self.mark(Areas::PANES | Areas::SIDEBAR);
    }

    pub fn unzoom(&mut self) -> bool {
        if self.zoomed.take().is_none() {
            return false;
        }
        self.mark(Areas::PANES);
        true
    }

    pub(crate) fn settle_zoom(&mut self) {
        let Some(id) = self.zoomed.as_deref() else { return };
        let stays = self.is_open(id) && self.pane_count() > 1 && self.focused_tab_id().as_deref() == Some(id);
        if !stays {
            self.zoomed = None;
        }
    }
}
