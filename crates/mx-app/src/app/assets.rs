use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

const FILES: &[(&str, &[u8])] = &[
    ("icons/claude.svg", include_bytes!("../../assets/icons/claude.svg")),
    ("icons/terminal.svg", include_bytes!("../../assets/icons/terminal.svg")),
    ("glyphs/open_with.svg", include_bytes!("../../assets/glyphs/open_with.svg")),
    ("glyphs/spin_arc.svg", include_bytes!("../../assets/glyphs/spin_arc.svg")),
    ("glyphs/window_minimize.svg", include_bytes!("../../assets/glyphs/window_minimize.svg")),
    ("glyphs/window_maximize.svg", include_bytes!("../../assets/glyphs/window_maximize.svg")),
    ("glyphs/window_restore.svg", include_bytes!("../../assets/glyphs/window_restore.svg")),
    ("glyphs/window_close.svg", include_bytes!("../../assets/glyphs/window_close.svg")),
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(FILES
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(FILES
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| SharedString::from(*p))
            .collect())
    }
}
