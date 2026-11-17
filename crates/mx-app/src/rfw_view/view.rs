use std::path::PathBuf;
use std::sync::Arc;

use gpui::{Context, Div, IntoElement, ParentElement, Render, Styled, Subscription, Window, div, px, relative};
use mx_rfw::{DynamicContent, Library, Widget, evaluate, parse_error_text, parse_library_file};
use mx_theme::{Palette, Theme, Typography};

use super::ActionSink;
use super::code_editor::CodeEditors;
use super::states::{FollowLink, WidgetStates};
use super::layout::{Ctx, DragNow, Fit, grow};
use super::menu::{self, HasMenu, OpenMenu};
use super::style::{UI_FONT, UI_LINE, hsla, rfw_theme};
use super::widgets_core::render;

pub fn mono_family() -> &'static str {
    "Hack"
}

pub fn text_root(d: Div, p: &Palette) -> Div {
    d.text_color(hsla(p.fg)).text_size(px(13.0)).font_family(UI_FONT).line_height(relative(UI_LINE))
}

pub struct RfwView {
    source: Option<String>,
    library: Option<Library>,
    error: Option<String>,
    root: String,
    content: DynamicContent,
    /// A árvore avaliada de `library` + `root` + `content`. Ela só muda quando
    /// um deles muda (os `set_*` e `merge_data` a jogam fora), e cada quadro de
    /// rolagem ou de hover remonta a tela: sem o cache, todo quadro avaliava o
    /// plugin inteiro de novo.
    tree: Option<Widget>,
    /// A `tree` atual já passou pelos `sync` dos editores e campos.
    tree_synced: bool,
    theme: Arc<Theme>,
    dir: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    sink: ActionSink,
    background: Option<gpui::Hsla>,
    pub(super) menu: Option<OpenMenu>,
    hoisted_menu: bool,
    pub(super) dragging: Option<DragNow>,
    editors: CodeEditors,
    widgets: WidgetStates,
    activation: Option<Subscription>,
    /// Quantas vezes a tela foi montada: os testes do cache (`plugin_ui/surface.rs`).
    #[cfg(test)]
    pub(crate) renders: usize,
    /// Quantas vezes a árvore foi avaliada: os testes do cache da árvore.
    #[cfg(test)]
    pub(crate) evaluations: usize,
}

impl HasMenu for RfwView {
    fn menu_slot(&mut self) -> &mut Option<OpenMenu> {
        &mut self.menu
    }
}

impl RfwView {
    pub fn new(theme: Arc<Theme>, typography: &Typography, dir: Option<PathBuf>, sink: ActionSink) -> Self {
        let mut content = DynamicContent::new();
        content.set_theme(rfw_theme(&theme, typography));
        Self {
            source: None,
            library: None,
            error: None,
            root: "root".into(),
            content,
            tree: None,
            tree_synced: false,
            theme,
            dir,
            data_dir: None,
            sink,
            background: None,
            menu: None,
            hoisted_menu: false,
            dragging: None,
            editors: CodeEditors::default(),
            widgets: WidgetStates::default(),
            activation: None,
            #[cfg(test)]
            renders: 0,
            #[cfg(test)]
            evaluations: 0,
        }
    }

    pub fn set_library(&mut self, text: &str) -> bool {
        if self.source.as_deref() == Some(text) {
            return false;
        }
        self.source = Some(text.to_owned());
        self.forget_tree();
        match parse_library_file(text) {
            Ok(lib) => {
                self.library = Some(lib);
                self.error = None;
            }
            Err(e) => self.error = Some(parse_error_text(&e)),
        }
        true
    }

    pub fn hoist_menu(&mut self) {
        self.hoisted_menu = true;
    }

    pub fn menu_overlay(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let theme = self.theme.clone();
        self.menu.as_ref().map(|m| menu::floating(&theme, cx, m).into_any_element())
    }

    pub fn set_root(&mut self, root: &str) -> bool {
        if self.root == root {
            return false;
        }
        root.clone_into(&mut self.root);
        self.forget_tree();
        true
    }

    pub fn merge_data(&mut self, data: &serde_json::Map<String, serde_json::Value>) {
        self.content.merge_plugin_data(data);
        self.forget_tree();
    }

    pub fn set_theme(&mut self, theme: Arc<Theme>, typography: &Typography) {
        self.content.set_theme(rfw_theme(&theme, typography));
        self.theme = theme;
        self.forget_tree();
    }

    fn forget_tree(&mut self) {
        self.tree = None;
        self.tree_synced = false;
    }

    pub fn set_dir(&mut self, dir: Option<PathBuf>) {
        self.dir = dir;
    }

    pub fn set_data_dir(&mut self, dir: Option<PathBuf>) {
        self.data_dir = dir;
    }

    pub(super) fn code_pane(&self, doc: &str) -> Option<gpui::Entity<super::code_pane::CodePane>> {
        self.editors.pane(doc).cloned()
    }

    pub fn set_plugin(&mut self, plugin_id: &str, follow: Option<FollowLink>) {
        plugin_id.clone_into(&mut self.widgets.plugin_id);
        self.widgets.follow = follow;
    }

    #[cfg(test)]
    pub(super) fn text_field(&self, id: &str) -> Option<gpui::Entity<super::text_input_widget::FieldView>> {
        self.widgets.inputs.field(id)
    }

    #[cfg(test)]
    pub(super) fn widget_states(&self) -> &WidgetStates {
        &self.widgets
    }

    pub fn set_background(&mut self, background: Option<gpui::Hsla>) {
        self.background = background;
    }
}

impl Render for RfwView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(test)]
        {
            self.renders += 1;
        }
        let _perf = crate::perf::span("rfw.render");
        if !cx.has_active_drag() {
            self.dragging = None;
        }
        if self.activation.is_none() {
            self.activation = Some(cx.observe_window_activation(window, |this, _, _| this.widgets.release_space()));
        }
        let p = self.theme.clone();
        let mut root = text_root(div(), &p).size_full().flex().flex_col();
        if let Some(bg) = self.background {
            root = root.bg(bg);
        }
        if let Some(e) = &self.error {
            return root
                .p(px(14.0))
                .text_size(px(11.5))
                .line_height(relative(1.45))
                .font_family(mono_family())
                .text_color(hsla(p.red))
                .child(e.clone())
                .into_any_element();
        }
        let Some(lib) = &self.library else { return root.into_any_element() };
        let tree: &Widget = match &mut self.tree {
            Some(tree) => {
                crate::perf::count("rfw.tree_cached");
                tree
            }
            slot @ None => {
                #[cfg(test)]
                {
                    self.evaluations += 1;
                }
                let _perf = crate::perf::span("rfw.evaluate");
                slot.insert(evaluate(lib, &self.root, &self.content))
            }
        };
        if !self.tree_synced {
            self.editors.sync(tree, &self.theme, &self.sink, window, cx);
            self.widgets.inputs.sync(tree, &self.theme, &self.sink, window, cx);
            self.tree_synced = true;
        }
        self.widgets.begin(window, cx);
        let mut ctx = Ctx {
            palette: &self.theme,
            dir: self.dir.as_deref(),
            data_dir: self.data_dir.as_deref(),
            sink: self.sink.clone(),
            view: cx.weak_entity(),
            ids: 0,
            drops: Vec::new(),
            dragging: self.dragging.clone(),
            turns: 0.0,
            interactive: true,
            bounded_w: true,
            editors: Some(&self.editors),
            widgets: Some(&self.widgets),
        };
        let body = {
            let _perf = crate::perf::span("rfw.build");
            grow(render(&mut ctx, tree, Fit::BOTH), 1.0)
        };
        self.widgets.finish();
        let overlay = self.menu.as_ref().filter(|_| !self.hoisted_menu).map(|m| menu::render(&p, cx, m));
        root.child(body).children(overlay).into_any_element()
    }
}
