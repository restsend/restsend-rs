//! Letter avatar generation (Go `avatars.LetterBuilder` parity) and topic
//! icon composition (member photo grid).

use image::imageops::FilterType;
use image::{Rgba, RgbaImage};
use std::collections::HashMap;
use std::sync::Mutex;

/// 8x8 bitmap glyphs for A-Z / 0-9 / '?' (public-domain font8x8 style,
/// one byte per row, LSB = leftmost pixel).
const FONT8X8: [&[u8; 8]; 38] = [
    b"\x00\x00\x00\x00\x00\x00\x00\x00", // unused slot
    // A-Z
    b"\x0C\x1E\x33\x33\x3F\x33\x33\x00", // A
    b"\x3F\x66\x66\x3E\x66\x66\x3F\x00", // B
    b"\x1C\x36\x63\x63\x63\x36\x1C\x00", // C
    b"\x1F\x33\x66\x66\x66\x33\x1F\x00", // D
    b"\x3F\x60\x60\x7E\x60\x60\x3F\x00", // E
    b"\x3F\x60\x60\x7E\x60\x60\x60\x00", // F
    b"\x1C\x36\x63\x63\x6F\x36\x1D\x00", // G
    b"\x33\x33\x33\x3F\x33\x33\x33\x00", // H
    b"\x1E\x0C\x0C\x0C\x0C\x0C\x1E\x00", // I
    b"\x07\x03\x03\x03\x63\x63\x3E\x00", // J
    b"\x33\x36\x6C\x78\x6C\x36\x33\x00", // K
    b"\x30\x30\x30\x30\x30\x30\x3F\x00", // L
    b"\x63\x77\x7F\x6B\x63\x63\x63\x00", // M
    b"\x63\x73\x7B\x6F\x67\x63\x63\x00", // N
    b"\x1C\x36\x63\x63\x63\x36\x1C\x00", // O
    b"\x3F\x66\x66\x3E\x60\x60\x60\x00", // P
    b"\x1C\x36\x63\x63\x6B\x36\x1D\x00", // Q
    b"\x3F\x66\x66\x3E\x6C\x36\x33\x00", // R
    b"\x1E\x60\x60\x1C\x06\x06\x3C\x00", // S
    b"\x7E\x5A\x18\x18\x18\x18\x3C\x00", // T
    b"\x33\x33\x33\x33\x33\x33\x1E\x00", // U
    b"\x33\x33\x33\x33\x33\x1E\x0C\x00", // V
    b"\x63\x63\x63\x6B\x7F\x77\x63\x00", // W
    b"\x63\x63\x36\x1C\x1C\x36\x63\x00", // X
    b"\x33\x33\x33\x1E\x0C\x0C\x1E\x00", // Y
    b"\x7F\x63\x31\x18\x4C\x66\x7F\x00", // Z
    // 0-9
    b"\x1E\x33\x73\x7B\x6F\x37\x1E\x00", // 0
    b"\x0C\x1C\x0C\x0C\x0C\x0C\x3F\x00", // 1
    b"\x1E\x33\x03\x06\x0C\x18\x3F\x00", // 2
    b"\x3F\x06\x0C\x06\x03\x33\x1E\x00", // 3
    b"\x06\x0E\x1E\x36\x7F\x06\x06\x00", // 4
    b"\x3F\x60\x7E\x03\x03\x33\x1E\x00", // 5
    b"\x1C\x30\x60\x7E\x63\x63\x3E\x00", // 6
    b"\x7F\x63\x06\x0C\x18\x18\x18\x00", // 7
    b"\x1E\x33\x33\x1E\x33\x33\x1E\x00", // 8
    b"\x1E\x33\x33\x3F\x03\x06\x3C\x00", // 9
    // '?'
    b"\x1E\x33\x03\x06\x0C\x00\x0C\x00",
];

fn glyph_index(ch: char) -> usize {
    let upper = ch.to_ascii_uppercase();
    match upper {
        'A'..='Z' => (upper as usize - 'A' as usize) + 1,
        '0'..='9' => (upper as usize - '0' as usize) + 27,
        _ => 37,
    }
}

/// Deterministic background color palette (Go uses a material palette).
const PALETTE: [(u8, u8, u8); 16] = [
    (244, 67, 54),
    (233, 30, 99),
    (156, 39, 176),
    (103, 58, 183),
    (63, 81, 181),
    (33, 150, 243),
    (3, 169, 244),
    (0, 188, 212),
    (0, 150, 136),
    (76, 175, 80),
    (139, 195, 74),
    (255, 235, 59),
    (255, 193, 7),
    (255, 152, 0),
    (121, 85, 72),
    (96, 125, 139),
];

pub fn palette_color(seed: &str) -> Rgba<u8> {
    let hash: u64 = seed.bytes().map(|b| b as u64).fold(0xcbf29ce484222325, |acc, b| {
        (acc ^ b as u64).wrapping_mul(0x100000001b3)
    });
    let (r, g, bl) = PALETTE[(hash % PALETTE.len() as u64) as usize];
    Rgba([r, g, bl, 255])
}

/// Render a square letter avatar PNG (white initial on a palette color).
pub fn letter_avatar_png(user_id: &str, size: u32) -> Vec<u8> {
    let size = size.clamp(32, 512);
    let mut canvas = RgbaImage::from_pixel(size, size, palette_color(user_id));

    let first = user_id
        .trim()
        .chars()
        .next()
        .filter(|c| !c.is_whitespace())
        .unwrap_or('?');
    let glyph = FONT8X8[glyph_index(first)];
    let white = Rgba([255u8, 255, 255, 255]);

    // scale the 8x8 glyph to fill ~60% of the canvas, centered
    let scale = ((size as f32 * 0.6) / 8.0).ceil().max(1.0) as u32;
    let glyph_size = 8 * scale;
    let offset_x = size.saturating_sub(glyph_size) / 2;
    let offset_y = size.saturating_sub(glyph_size) / 2;
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..8 {
            if bits & (1 << col) != 0 {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let x = offset_x + col as u32 * scale + dx;
                        let y = offset_y + row as u32 * scale + dy;
                        if x < size && y < size {
                            canvas.put_pixel(x, y, white);
                        }
                    }
                }
            }
        }
    }

    let mut buf = std::io::Cursor::new(Vec::new());
    canvas
        .write_to(&mut buf, image::ImageFormat::Png)
        .expect("png encode");
    buf.into_inner()
}

/// Compose member avatars into a grid icon (1: full, 2: 2x1, 3-4: 2x2,
/// 5-6: 3x2, 7-9: 3x3). Missing slots fall back to the background color.
pub fn pack_grid(cells: Vec<image::DynamicImage>, size: u32) -> Vec<u8> {
    let size = size.clamp(64, 1024);
    let mut canvas = RgbaImage::from_pixel(size, size, Rgba([235, 238, 242, 255]));
    let count = cells.len().max(1);
    let (cols, rows) = match count {
        1 => (1, 1),
        2 => (2, 1),
        3..=4 => (2, 2),
        5..=6 => (3, 2),
        _ => (3, 3),
    };
    let cell_w = size / cols as u32;
    let cell_h = size / rows as u32;

    for (idx, img) in cells.iter().take((cols * rows) as usize).enumerate() {
        let col = (idx % cols) as u32;
        let row = (idx / cols) as u32;
        let resized = img.resize_exact(cell_w, cell_h, FilterType::Lanczos3).to_rgba8();
        image::imageops::overlay(
            &mut canvas,
            &resized,
            (col * cell_w) as i64,
            (row * cell_h) as i64,
        );
    }

    let mut buf = std::io::Cursor::new(Vec::new());
    canvas
        .write_to(&mut buf, image::ImageFormat::Png)
        .expect("png encode");
    buf.into_inner()
}

/// Small in-process cache for generated letter avatars (id -> png bytes).
#[derive(Default)]
pub struct AvatarCache {
    inner: Mutex<HashMap<(String, u32), Vec<u8>>>,
}

const CACHE_LIMIT: usize = 4096;

impl AvatarCache {
    pub fn get_or_insert(&self, key: (String, u32), generate: impl FnOnce() -> Vec<u8>) -> Vec<u8> {
        let mut guard = self.inner.lock().unwrap();
        if let Some(bytes) = guard.get(&key) {
            return bytes.clone();
        }
        let bytes = generate();
        if guard.len() >= CACHE_LIMIT {
            guard.clear();
        }
        guard.insert(key, bytes.clone());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;

    #[test]
    fn letter_avatar_renders_png_of_requested_size() {
        let png = letter_avatar_png("alice", 128);
        let img = image::load_from_memory(&png).unwrap();
        assert_eq!(img.dimensions(), (128, 128));
        // letter pixel in the center region must be white-ish
        let center = img.get_pixel(64, 64);
        assert!(center.0[0] > 200 || center.0 != palette_color("alice").0);
    }

    #[test]
    fn palette_is_deterministic() {
        assert_eq!(palette_color("bob"), palette_color("bob"));
    }

    #[test]
    fn pack_grid_handles_counts() {
        let one = pack_grid(vec![image::DynamicImage::new_rgba8(10, 10)], 96);
        assert_eq!(image::load_from_memory(&one).unwrap().dimensions(), (96, 96));
        let nine: Vec<image::DynamicImage> = (0..9)
            .map(|_| image::DynamicImage::new_rgba8(10, 10))
            .collect();
        let grid = pack_grid(nine, 96);
        assert_eq!(image::load_from_memory(&grid).unwrap().dimensions(), (96, 96));
    }
}
