//! Terminal QR rendering helpers for remote companion handoff.

use anyhow::Result;
use qrcode::{render::unicode, QrCode};

pub fn render_terminal_qr(text: &str) -> Result<String> {
    let code = QrCode::new(text.as_bytes())?;
    Ok(code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Dark)
        .light_color(unicode::Dense1x2::Light)
        .quiet_zone(true)
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_qr_contains_unicode_blocks() {
        let rendered = render_terminal_qr("aiterminal://pair?payload=test").unwrap();
        assert!(rendered.contains('█') || rendered.contains('▀') || rendered.contains('▄'));
        assert!(rendered.lines().count() > 4);
    }
}
