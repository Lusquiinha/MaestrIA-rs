use gpui::{
    AnyView, AppContext, Context, Entity, IntoElement, ParentElement, Render, StyleRefinement, Styled, TestAppContext,
    Window, WindowHandle, deferred, div, px,
};

struct Leaf {
    renders: usize,
}

impl Render for Leaf {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders += 1;
        div().size_full().child("leaf ▲ ◆ ● ✚ ★ ◉ ação")
    }
}

fn leaf_slot(leaf: &Entity<Leaf>) -> AnyView {
    AnyView::from(leaf.clone()).cached(StyleRefinement::default().w(px(200.)).h(px(20.)))
}

fn lines(count: usize) -> impl Iterator<Item = gpui::Div> {
    (0..count).map(|i| div().h(px(20.)).child(format!("linha {i} — título com acento")))
}

fn renders(leaf: &Entity<Leaf>, cx: &mut TestAppContext) -> usize {
    cx.read(|cx| leaf.read(cx).renders)
}

fn draw<V: 'static>(window: WindowHandle<V>, cx: &mut TestAppContext) {
    cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear()).expect("draw window");
}

struct HidingRoot {
    hidden: bool,
    lines: usize,
    leaf: Entity<Leaf>,
}

impl Render for HidingRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut slot = div().w(px(200.)).h(px(20.)).child(leaf_slot(&self.leaf));
        if self.hidden {
            slot = slot.invisible();
        }
        div().size_full().child(slot).children(lines(self.lines))
    }
}

#[gpui::test]
fn a_cached_view_shown_again_after_an_invisible_frame_does_not_reuse_a_stale_paint(cx: &mut TestAppContext) {
    let leaf = cx.new(|_| Leaf { renders: 0 });
    let window = cx.add_window(|_, _| HidingRoot { hidden: false, lines: 8, leaf: leaf.clone() });
    draw(window, cx);

    window
        .update(cx, |root, _, cx| {
            root.hidden = true;
            root.lines = 0;
            cx.notify();
        })
        .unwrap();
    assert_eq!(renders(&leaf, cx), 1);

    window
        .update(cx, |root, _, cx| {
            root.hidden = false;
            cx.notify();
        })
        .unwrap();
    assert_eq!(renders(&leaf, cx), 2, "a pintura do quadro em que estava visível já não existe");

    window.update(cx, |_, _, cx| cx.notify()).unwrap();
    assert_eq!(renders(&leaf, cx), 2, "visível e pintada no quadro anterior, reaproveita o cache");
}

struct Holder {
    leaf: Entity<Leaf>,
}

impl Render for Holder {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(deferred(leaf_slot(&self.leaf)))
    }
}

struct DeferringRoot {
    lines: usize,
    holder: Entity<Holder>,
}

impl Render for DeferringRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(AnyView::from(self.holder.clone()).cached(StyleRefinement::default().w(px(200.)).h(px(20.))))
            .children(lines(self.lines))
    }
}

#[gpui::test]
fn a_deferred_cached_view_carried_by_its_cached_holder_renders_again(cx: &mut TestAppContext) {
    let leaf = cx.new(|_| Leaf { renders: 0 });
    let holder = cx.new(|_| Holder { leaf: leaf.clone() });
    let window = cx.add_window(|_, _| DeferringRoot { lines: 8, holder: holder.clone() });
    draw(window, cx);

    window
        .update(cx, |root, _, cx| {
            root.lines = 0;
            cx.notify();
        })
        .unwrap();
    assert_eq!(renders(&leaf, cx), 1);

    holder.update(cx, |_, cx| cx.notify());
    assert_eq!(renders(&leaf, cx), 2, "o estado levado pelo holder aponta para um quadro que já não existe");
}
