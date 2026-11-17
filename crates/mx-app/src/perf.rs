//! Medição de desempenho, ligada por `MX_PERF=1`.
//!
//! Desligada, cada `span` custa só a leitura de um `OnceLock`. Ligada, junta o
//! tempo de cada etiqueta e, a cada dois segundos de uso, escreve no log uma
//! linha por etiqueta: quantas vezes rodou, média, p95 e máximo. A tela parada
//! não escreve nada.
//!
//! ```sh
//! MX_PERF=1 RUST_LOG=info ./target/release/maestria
//! ```
//!
//! `frame` é o `Window::draw` inteiro do gpui (montar as views sujas, medir e
//! pintar); o resto são pedaços dele. A 120 Hz o quadro tem 8,3 ms.
//!
//! As etiquetas `gpui.*` vêm de dentro do gpui, pelo `set_perf_observer`:
//! `request_layout` (os `render` das views), `layout` (taffy da raiz),
//! `prepaint`, `overlays` (deferred, tooltip, prompt), `paint`, `finish`,
//! `focus` e `present`; `taffy` (cada cálculo de layout, também os de dentro
//! do `prepaint`, então se sobrepõe a `layout` e `prepaint`); e o trabalho de
//! primeira vez: `shape` (texto que não estava no cache), `font.load`,
//! `raster.glyph|emoji|svg` e `atlas.image`. `asset.load` são os arquivos
//! embutidos do app.
//!
//! Um quadro acima de [`SLOW_FRAME`] escreve na hora uma linha `perf slow`
//! com o tempo de cada etiqueta dentro dele, para saber onde o tempo foi.
//!
//! `MX_PERF_TOUR=1` abre sozinho as configurações, o catálogo e cada plugin do
//! rail (ver `app::perf_tour`), para comparar antes e depois.

use std::cell::RefCell;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(2);
/// Quadro que vale uma linha `perf slow` com o detalhe.
const SLOW_FRAME: Duration = Duration::from_millis(50);

pub fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("MX_PERF").is_ok_and(|v| !v.is_empty() && v != "0"))
}

/// Liga a medição do quadro do gpui, se `MX_PERF` estiver ligado.
pub fn install() {
    if enabled() {
        gpui::set_perf_observer(record);
    }
}

/// Mede do `span` até o fim do escopo.
#[must_use]
pub fn span(label: &'static str) -> Span {
    Span { label, start: enabled().then(Instant::now) }
}

pub struct Span {
    label: &'static str,
    start: Option<Instant>,
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            record(self.label, start.elapsed());
        }
    }
}

/// Conta um evento sem tempo (ex.: a árvore do plugin veio do cache).
pub fn count(label: &'static str) {
    if enabled() {
        record(label, Duration::ZERO);
    }
}

#[derive(Default)]
struct Row {
    samples: Vec<Duration>,
}

struct Stats {
    since: Instant,
    rows: Vec<(&'static str, Row)>,
    /// O que rodou desde o último `frame`: (etiqueta, vezes, tempo somado).
    in_frame: Vec<(&'static str, u32, Duration)>,
}

thread_local! {
    static STATS: RefCell<Stats> =
        RefCell::new(Stats { since: Instant::now(), rows: Vec::new(), in_frame: Vec::new() });
}

fn record(label: &'static str, took: Duration) {
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        if label == "frame" {
            let parts = std::mem::take(&mut s.in_frame);
            if took >= SLOW_FRAME {
                log::info!("perf {}", slow_line(took, parts));
            }
        } else {
            match s.in_frame.iter_mut().find(|(l, _, _)| *l == label) {
                Some((_, n, sum)) => {
                    *n += 1;
                    *sum += took;
                }
                None => s.in_frame.push((label, 1, took)),
            }
        }
        if s.rows.is_empty() {
            s.since = Instant::now();
        }
        match s.rows.iter_mut().find(|(l, _)| *l == label) {
            Some((_, row)) => row.samples.push(took),
            None => s.rows.push((label, Row { samples: vec![took] })),
        }
        if s.since.elapsed() >= WINDOW {
            let secs = s.since.elapsed().as_secs_f32();
            for (label, row) in s.rows.drain(..) {
                log::info!("perf {}", line(label, row, secs));
            }
        }
    });
}

fn line(label: &str, mut row: Row, secs: f32) -> String {
    let n = row.samples.len();
    let per_sec = n as f32 / secs;
    if row.samples.iter().all(Duration::is_zero) {
        return format!("{label:<16} n={n:<5} ({per_sec:.0}/s)");
    }
    row.samples.sort_unstable();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    let total: Duration = row.samples.iter().sum();
    let p95 = row.samples[(n * 95 / 100).min(n - 1)];
    let max = row.samples[n - 1];
    format!(
        "{label:<16} n={n:<5} ({per_sec:.0}/s) avg={:.2}ms p95={:.2}ms max={:.2}ms",
        ms(total) / n as f64,
        ms(p95),
        ms(max)
    )
}

/// `slow frame=340.2ms gpui.raster.svg=310.0ms(x12) gpui.paint=325.1ms ...`,
/// do mais caro para o mais barato; contadores sem tempo ficam de fora.
fn slow_line(frame: Duration, mut parts: Vec<(&'static str, u32, Duration)>) -> String {
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    parts.retain(|(_, _, sum)| !sum.is_zero());
    parts.sort_by_key(|p| std::cmp::Reverse(p.2));
    let mut out = format!("slow frame={:.1}ms", ms(frame));
    for (label, n, sum) in parts {
        out.push_str(&format!(" {label}={:.1}ms", ms(sum)));
        if n > 1 {
            out.push_str(&format!("(x{n})"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_reports_avg_p95_and_max() {
        let samples = (1..=100).map(Duration::from_millis).collect();
        let text = line("frame", Row { samples }, 2.0);
        assert_eq!(text, "frame            n=100   (50/s) avg=50.50ms p95=96.00ms max=100.00ms");
    }

    #[test]
    fn slow_line_sorts_by_cost_and_skips_counters() {
        let ms = Duration::from_millis;
        let parts = vec![
            ("gpui.paint", 1, ms(20)),
            ("rfw.tree_cached", 3, Duration::ZERO),
            ("gpui.raster.svg", 12, ms(300)),
        ];
        assert_eq!(
            slow_line(ms(340), parts),
            "slow frame=340.0ms gpui.raster.svg=300.0ms(x12) gpui.paint=20.0ms"
        );
    }

    #[test]
    fn counters_have_no_timing() {
        let text = line("rfw.cached", Row { samples: vec![Duration::ZERO; 4] }, 2.0);
        assert_eq!(text, "rfw.cached       n=4     (2/s)");
    }
}
