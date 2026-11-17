//! `ListView` vertical comprida montada só onde aparece.
//!
//! Rolar avisa a tela do plugin, e ela era montada, medida e pintada inteira a
//! cada quadro: 300 linhas de uma lista custavam ~7,5 ms por tick de rolagem,
//! quase o quadro todo a 120 Hz. Aqui só entram os itens visíveis e uma tela de
//! folga acima e abaixo; o resto vira dois espaçadores com a altura que os itens
//! teriam (a medida do último quadro em que apareceram, ou uma estimativa).
//!
//! Cuidados:
//! - o primeiro quadro monta tudo, pra medir. Se a lista usar estado que
//!   depende da ordem de montagem (sprites, zoom, atalhos, âncoras: o `stamp`
//!   mudou), ela fica inteira daí em diante, como era;
//! - cada item tem a sua faixa de ids de elemento, e o contador volta ao que
//!   era depois da lista: o que vem depois não muda de id conforme a rolagem;
//! - quando a altura medida de um item acima difere da estimada, a rolagem é
//!   corrigida pra o item do topo não pular.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use gpui::{AlignItems, ScrollHandle, StatefulInteractiveElement, Styled, div, point, prelude::*, px};
use mx_rfw::decoders::Axis;
use mx_rfw::widgets::core_widgets::ScrollList;

use super::layout::{Ctx, El, Fit, rigid, stretch_self};
use super::style::{pxf, set_padding};
use super::widgets_core::child;

/// Abaixo disso, montar tudo é barato e não vale o risco.
pub const MIN_ITEMS: usize = 40;
/// Telas de folga acima e abaixo do que aparece.
const OVERSCAN: f32 = 1.0;
/// Altura de um item nunca medido, sem `itemExtent` nem nenhum medido.
const GUESS: f32 = 32.0;
/// Altura da tela antes da primeira medida.
const FIRST_VIEWPORT: f32 = 1200.0;

#[derive(Default)]
struct State {
    handle: ScrollHandle,
    heights: Vec<Option<f32>>,
    /// Os itens montados no último quadro, na ordem dos filhos (depois do
    /// espaçador de cima).
    shown: Range<usize>,
    measured_once: bool,
    /// A lista usa estado por ordem de montagem: fica inteira.
    eager: bool,
}

/// Por tela (`WidgetStates::owner`) e lista.
type Registry = HashMap<(u64, String), Rc<RefCell<State>>>;

thread_local! {
    static STATES: RefCell<Registry> = RefCell::new(HashMap::new());
}

pub fn forget_owner(owner: u64) {
    STATES.with(|s| s.borrow_mut().retain(|(o, _), _| *o != owner));
}

fn state(owner: u64, key: String) -> Rc<RefCell<State>> {
    STATES.with(|s| s.borrow_mut().entry((owner, key)).or_default().clone())
}

/// A lista montada só onde aparece, ou `None` quando ela tem que ser montada
/// inteira (curta, invertida, horizontal, grade, `shrinkWrap`, dentro de uma
/// seleção, ou com estado por ordem de montagem).
pub fn list(cx: &mut Ctx, l: &ScrollList, fit: Fit) -> Option<El> {
    if l.grid_delegate.is_some()
        || l.reverse
        || l.shrink_wrap
        || l.scroll_direction != Axis::Vertical
        || l.children.len() < MIN_ITEMS
    {
        return None;
    }
    let states = cx.widgets?;
    if states.selection().is_some() {
        return None;
    }
    // O id que o contêiner vai ganhar: estável enquanto o que vem antes for.
    let key = match &l.id {
        Some(id) => format!("id:{id}"),
        None => format!("el:{}", cx.ids + 1),
    };
    let slot = state(states.owner, key);
    let mut st = slot.borrow_mut();
    if st.eager {
        return None;
    }
    let handle = match &l.id {
        Some(id) => states.scroll(id),
        None => st.handle.clone(),
    };
    let n = l.children.len();
    let fixed = l.item_extent.map(|e| f32::from(pxf(e)));
    measure(&mut st, &handle, n, fixed);
    let guess = fixed.unwrap_or_else(|| average(&st.heights));
    let heights: Vec<f32> = st.heights.iter().map(|h| h.unwrap_or(guess)).collect();
    let range = if st.measured_once { visible(&heights, &handle) } else { 0..n };

    let before = states.stamp();
    let mut el = cx.el().flex().flex_col().min_w(px(0.0)).min_h(px(0.0)).overflow_y_scroll();
    el.style().align_items = Some(AlignItems::Stretch);
    el = el.track_scroll(&handle);
    if let Some(p) = &l.padding {
        set_padding(el.style(), p);
    }
    let container = cx.ids;
    let above: f32 = heights[..range.start].iter().sum();
    let below: f32 = heights[range.end..].iter().sum();
    el = el.child(div().flex_none().h(px(above)));
    for i in range.clone() {
        cx.ids = item_ids(container, i);
        let mut item = rigid(cx.within(true, |cx| child(cx, l.children[i].clone(), Fit { w: true, h: false })));
        if let Some(h) = fixed {
            item = item.h(px(h));
        }
        el = el.child(item);
    }
    el = el.child(div().flex_none().h(px(below)));
    // Depois da lista, o contador segue de onde o contêiner parou, quantos
    // itens tenham sido montados.
    cx.ids = container;

    if states.stamp() != before {
        st.eager = true;
    }
    st.shown = range;
    st.measured_once = true;
    Some(if fit.any() { stretch_self(el) } else { el })
}

/// Guarda as alturas que o último quadro mediu e corrige a rolagem pra o item
/// do topo ficar onde estava, se algum acima mudou de altura.
fn measure(st: &mut State, handle: &ScrollHandle, n: usize, fixed: Option<f32>) {
    st.heights.resize(n, None);
    if let Some(h) = fixed {
        st.heights.iter_mut().for_each(|x| *x = Some(h));
        return;
    }
    let guess = average(&st.heights);
    let old: Vec<f32> = st.heights.iter().map(|h| h.unwrap_or(guess)).collect();
    let shown = st.shown.start.min(n)..st.shown.end.min(n);
    for (k, i) in shown.enumerate() {
        if let Some(b) = handle.bounds_for_item(k + 1) {
            st.heights[i] = Some(f32::from(b.size.height));
        }
    }
    let guess = average(&st.heights);
    let new: Vec<f32> = st.heights.iter().map(|h| h.unwrap_or(guess)).collect();
    let top = -f32::from(handle.offset().y);
    if let Some(fixed_top) = anchored(&old, &new, top)
        && (fixed_top - top).abs() > 0.5
    {
        handle.set_offset(point(handle.offset().x, px(-fixed_top)));
    }
}

/// O topo da rolagem que mantém no lugar o item que estava no topo (`top` nas
/// alturas `old`), agora com as alturas `new`.
fn anchored(old: &[f32], new: &[f32], top: f32) -> Option<f32> {
    if top <= 0.0 {
        return None;
    }
    let mut start = 0.0;
    for (i, h) in old.iter().enumerate() {
        if start + h > top {
            let into = top - start;
            let new_start: f32 = new[..i].iter().sum();
            return Some(new_start + into.min(new[i]));
        }
        start += h;
    }
    None
}

/// Os itens que aparecem, mais `OVERSCAN` telas de folga dos dois lados.
fn visible(heights: &[f32], handle: &ScrollHandle) -> Range<usize> {
    let top = -f32::from(handle.offset().y);
    let viewport = match f32::from(handle.bounds().size.height) {
        h if h >= 1.0 => h,
        _ => FIRST_VIEWPORT,
    };
    window(heights, top, viewport)
}

fn window(heights: &[f32], top: f32, viewport: f32) -> Range<usize> {
    let from = top - viewport * OVERSCAN;
    let to = top + viewport * (1.0 + OVERSCAN);
    let (mut start, mut end) = (heights.len(), heights.len());
    let mut y = 0.0;
    for (i, h) in heights.iter().enumerate() {
        if start == heights.len() && y + h > from {
            start = i;
        }
        if y >= to {
            end = i;
            break;
        }
        y += h;
    }
    start.min(end)..end
}

fn average(heights: &[Option<f32>]) -> f32 {
    let (sum, n) = heights.iter().flatten().fold((0.0, 0), |(s, n), h| (s + h, n + 1));
    if n == 0 { GUESS } else { sum / n as f32 }
}

/// Uma faixa de ids só do item `i`, longe do contador normal.
fn item_ids(container: u64, i: usize) -> u64 {
    let mut x = container.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (i as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 31;
    (x | 1 << 63) & !0xF_FFFF
}

/// Os itens montados no último quadro de cada lista virtual: os testes.
#[cfg(test)]
pub fn shown_now() -> Vec<Range<usize>> {
    STATES.with(|s| s.borrow().values().map(|st| st.borrow().shown.clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_covers_the_screen_and_one_screen_each_side() {
        let heights = vec![10.0; 100];
        assert_eq!(window(&heights, 0.0, 50.0), 0..10);
        assert_eq!(window(&heights, 500.0, 50.0), 45..60);
        assert_eq!(window(&heights, 960.0, 50.0), 91..100);
    }

    #[test]
    fn a_taller_item_above_keeps_the_top_item_in_place() {
        let old = vec![10.0; 10];
        let mut new = old.clone();
        new[1] = 30.0;
        // O topo estava 5 px dentro do item 3 (30..40): continua.
        assert_eq!(anchored(&old, &new, 35.0), Some(55.0));
        assert_eq!(anchored(&old, &new, 0.0), None);
    }

    #[test]
    fn items_get_ids_far_from_each_other_and_from_the_counter() {
        let a = item_ids(7, 0);
        let b = item_ids(7, 1);
        assert_ne!(a, b);
        assert!(a > 1 << 62 && b > 1 << 62);
        assert_eq!(a & 0xF_FFFF, 0);
    }
}
