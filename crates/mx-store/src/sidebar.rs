use mx_core::model::sidebar::RowKey;
use mx_core::model::{Folder, Tint, Workspace, new_workspace_id};
use mx_core::paths::expand_home_with;
use mx_core::text::last_segment;
use mx_sys::code_workspace::{self, CodeWorkspace, WorkspaceImport};

use crate::changes::Areas;
use crate::msg::Done;
use crate::restore::is_dir;
use crate::store::Store;

impl Store {
    pub fn folders(&self) -> &[Folder] {
        &self.folders
    }

    pub fn loose(&self) -> &Folder {
        &self.loose
    }

    fn expand(&self, raw: &str) -> String {
        expand_home_with(raw, self.opts.home.as_deref())
    }

    pub fn add_folder(&mut self, raw: &str) {
        let expanded = self.expand(raw.trim());
        let path = match expanded.trim_end_matches('/') {
            "" => expanded.clone(),
            p => p.to_owned(),
        };
        let git = self.git.clone();
        self.spawn(async move {
            if let Some(root) = git.main_root(&path).await {
                return Done::FolderFound { path, root, repos: Vec::new() };
            }
            let mut repos: Vec<String> = Vec::new();
            for child in repo_children(&path) {
                let root = git.main_root(&child).await.unwrap_or(child);
                if !repos.contains(&root) {
                    repos.push(root);
                }
            }
            Done::FolderFound { root: path.clone(), path, repos }
        });
    }

    pub(crate) fn folder_found(&mut self, path: &str, root: &str, repos: Vec<String>) {
        if !repos.is_empty() {
            self.adopt_repos_of(path, &repos);
            return;
        }
        match self.adopt(root) {
            None => self.show_banner_sticky(&format!("Pasta não encontrada: {path}")),
            Some(false) => {}
            Some(true) => {
                self.save();
                self.refresh_git();
                self.mark(Areas::SIDEBAR);
            }
        }
    }

    fn adopt(&mut self, root: &str) -> Option<bool> {
        if self.find_folder(root).is_some() {
            return Some(false);
        }
        if !is_dir(root) {
            return None;
        }
        self.folders.push(Folder::new(root, last_segment(root)));
        Some(true)
    }

    pub fn import_workspace(&mut self, raw: &str) {
        let raw = raw.to_owned();
        let home = self.opts.home.clone();
        let git = self.git.clone();
        self.spawn(async move {
            let ws = code_workspace::read_with(&raw, home.as_deref()).await;
            let found = match ws {
                None => None,
                Some(ws) => {
                    let mut roots = Vec::with_capacity(ws.folders.len());
                    for f in &ws.folders {
                        roots.push(git.main_root(&f.path).await.unwrap_or_else(|| f.path.clone()));
                    }
                    Some((ws, roots))
                }
            };
            Done::WorkspaceRead { raw, found }
        });
    }

    pub fn last_import(&self) -> Option<&WorkspaceImport<String>> {
        self.last_import.as_ref()
    }

    pub(crate) fn workspace_read(&mut self, raw: &str, found: Option<(CodeWorkspace, Vec<String>)>) {
        let Some((ws, roots)) = found else {
            let shown = self.expand(raw.trim());
            self.show_banner_sticky(&format!("Não consegui ler esse workspace: {shown}"));
            return;
        };
        if ws.folders.is_empty() {
            self.show_banner_sticky(&format!("O workspace \"{}\" não lista nenhuma pasta", ws.name));
            return;
        }
        let (mut added, mut already, mut missing) = (Vec::new(), Vec::new(), Vec::new());
        for (entry, root) in ws.folders.iter().zip(&roots) {
            match self.adopt(root) {
                None => missing.push(entry.path.clone()),
                Some(false) => already.push(root.clone()),
                Some(true) => {
                    if let Some(name) = &entry.name
                        && *root == entry.path
                        && let Some(f) = self.folder_mut(root)
                    {
                        f.name = name.clone();
                    }
                    added.push(root.clone());
                }
            }
        }
        let adopted: Vec<String> = added.iter().chain(&already).cloned().collect();
        if !adopted.is_empty() {
            let at = self.workspace_slot(&ws.name, Slot::CodeWorkspace(&ws.path));
            for root in &adopted {
                self.workspaces[at].add_root(root);
            }
        }
        let result = WorkspaceImport { workspace: ws, added, already, missing };
        self.save();
        if !result.added.is_empty() {
            self.refresh_git();
        }
        self.show_banner_with(&result.summary(), result.sticky());
        self.last_import = Some(result);
        self.mark(Areas::SIDEBAR);
    }

    fn adopt_repos_of(&mut self, path: &str, repos: &[String]) {
        let parent_added = match self.adopt(path) {
            None => {
                self.show_banner_sticky(&format!("Pasta não encontrada: {path}"));
                return;
            }
            Some(added) => added,
        };
        let name = last_segment(path).to_owned();
        let (mut added, mut already) = (0, 0);
        let mut adopted = Vec::with_capacity(repos.len());
        for root in repos {
            match self.adopt(root) {
                None => continue,
                Some(true) => added += 1,
                Some(false) => already += 1,
            }
            adopted.push(root);
        }
        let at = self.workspace_slot(&name, Slot::Root(path));
        let w = &mut self.workspaces[at];
        w.folder_roots.retain(|r| r != path);
        w.collapsed_folders.retain(|r| r != path);
        for root in adopted {
            w.add_root(root);
        }
        self.save();
        if parent_added || added > 0 {
            self.refresh_git();
        }
        self.show_banner(&code_workspace::summary(&name, added, already, 0));
        self.mark(Areas::SIDEBAR);
    }

    fn workspace_slot(&mut self, name: &str, slot: Slot) -> usize {
        let found = self.workspaces.iter().position(|w| match slot {
            Slot::CodeWorkspace(p) => w.code_workspace_path.as_deref() == Some(p),
            Slot::Root(r) => w.root.as_deref() == Some(r),
        });
        if let Some(i) = found {
            return i;
        }
        self.pin_root_order();
        let mut w = Workspace::new(new_workspace_id(), name);
        match slot {
            Slot::CodeWorkspace(p) => w.code_workspace_path = Some(p.to_owned()),
            Slot::Root(r) => w.root = Some(r.to_owned()),
        }
        self.root_order.push(RowKey::Workspace(w.id.clone()).key());
        self.workspaces.push(w);
        self.workspaces.len() - 1
    }

    fn edit_folder(&mut self, root: &str, edit: impl FnOnce(&mut Folder)) {
        let Some(f) = self.folder_mut(root) else { return };
        edit(f);
        self.save();
        self.mark(Areas::SIDEBAR);
    }

    pub fn set_folder_tint(&mut self, root: &str, tint: Option<Tint>) {
        self.edit_folder(root, |f| f.tint = tint);
    }

    pub fn rename_folder(&mut self, root: &str, name: &str) {
        self.edit_folder(root, |f| f.name = name.to_owned());
    }

    pub fn toggle_collapsed(&mut self, root: &str) {
        self.edit_folder(root, |f| f.collapsed = !f.collapsed);
    }

    pub fn remove_folder(&mut self, root: &str) {
        let Some(at) = self.folders.iter().position(|f| f.root == root) else {
            return;
        };
        self.folders.remove(at);
        for w in &mut self.workspaces {
            w.folder_roots.retain(|r| r != root);
            w.collapsed_folders.retain(|r| r != root);
            if w.root.as_deref() == Some(root) {
                w.root = None;
            }
        }
        let key = RowKey::Folder(root.to_owned()).key();
        self.root_order.retain(|k| *k != key);
        self.features.retain(|f| f.folder_root != root);
        let going: Vec<String> = self
            .tabs
            .iter()
            .filter(|t| t.folder_root == root && !t.loose)
            .map(|t| t.id.clone())
            .collect();
        for id in going {
            self.close_tab(&id);
        }
        self.save();
        self.mark(Areas::SIDEBAR);
    }

    pub fn focused_folder(&self) -> &Folder {
        if let Some(t) = self.focused_tab() {
            return self.folder_of(t);
        }
        self.folders.first().unwrap_or(&self.loose)
    }

    pub fn worktrees_of(&self, root: &str) -> &[mx_core::model::WorktreeInfo] {
        self.worktrees.get(root).map_or(&[], Vec::as_slice)
    }
}

#[derive(Clone, Copy)]
enum Slot<'a> {
    CodeWorkspace(&'a str),
    Root(&'a str),
}

fn repo_children(path: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    let mut found: Vec<String> = entries
        .flatten()
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join(".git").exists())
        .filter_map(|p| p.to_str().map(str::to_owned))
        .collect();
    found.sort();
    found
}
