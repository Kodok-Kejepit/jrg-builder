// src/canvas/inti.rs — inti gambar `Canvas::`: warna, huruf 5x7, framebuffer. Tanpa `std`: SALINAN PERSISNYA
// di `runtime/src/canvas/inti.rs` (runtime LLVM), dijaga tes, jadi biner `--emit-llvm` dan kernel `--bebas`
// menggambar dengan kode yang sama dengan interpreter dan VM.
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use alloc::collections::BTreeSet;

/// Batas elemen sebuah kanvas (piksel), sama dengan batas larik di tiga mesin (`interpreter::MAKS_ELEMEN`).
pub const MAKS_ELEMEN: i128 = 16 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Colour — ARGB u32 (minifb format)
// ---------------------------------------------------------------------------

/// Warna dalam format ARGB u32 (0xAARRGGBB), compatible dengan minifb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Colour {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Colour { r, g, b, a: 255 } }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self { Colour { r, g, b, a } }
    pub const BLACK:   Colour = Colour::rgb(0, 0, 0);
    pub const WHITE:   Colour = Colour::rgb(255, 255, 255);
    pub const RED:     Colour = Colour::rgb(255, 0, 0);
    pub const GREEN:   Colour = Colour::rgb(0, 255, 0);
    pub const BLUE:    Colour = Colour::rgb(0, 0, 255);
    pub const YELLOW:  Colour = Colour::rgb(255, 255, 0);
    pub const CYAN:    Colour = Colour::rgb(0, 255, 255);
    pub const MAGENTA: Colour = Colour::rgb(255, 0, 255);
    pub const ORANGE:  Colour = Colour::rgb(255, 165, 0);

    pub fn to_argb_u32(self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    pub fn from_argb_u32(v: u32) -> Self {
        Colour {
            a: ((v >> 24) & 0xFF) as u8,
            r: ((v >> 16) & 0xFF) as u8,
            g: ((v >>  8) & 0xFF) as u8,
            b: (v         & 0xFF) as u8,
        }
    }

    pub fn blend_over(self, bg: Colour) -> Colour {
        if self.a == 255 { return self; }
        if self.a == 0   { return bg; }
        let alpha = self.a as u32;
        let inv   = 255 - alpha;
        Colour {
            r: ((self.r as u32 * alpha + bg.r as u32 * inv) / 255) as u8,
            g: ((self.g as u32 * alpha + bg.g as u32 * inv) / 255) as u8,
            b: ((self.b as u32 * alpha + bg.b as u32 * inv) / 255) as u8,
            a: 255,
        }
    }
}

// ---------------------------------------------------------------------------
// Pixel-font 5×7
// ---------------------------------------------------------------------------

pub(crate) static FONT_5X7: &[u8] = &[
    0x00,0x00,0x00,0x00,0x00, // ' '
    0x00,0x00,0x5F,0x00,0x00, // '!'
    0x00,0x07,0x00,0x07,0x00, // '"'
    0x14,0x7F,0x14,0x7F,0x14, // '#'
    0x24,0x2A,0x7F,0x2A,0x12, // '$'
    0x23,0x13,0x08,0x64,0x62, // '%'
    0x36,0x49,0x55,0x22,0x50, // '&'
    0x00,0x05,0x03,0x00,0x00, // '''
    0x00,0x1C,0x22,0x41,0x00, // '('
    0x00,0x41,0x22,0x1C,0x00, // ')'
    0x14,0x08,0x3E,0x08,0x14, // '*'
    0x08,0x08,0x3E,0x08,0x08, // '+'
    0x00,0x50,0x30,0x00,0x00, // ','
    0x08,0x08,0x08,0x08,0x08, // '-'
    0x00,0x60,0x60,0x00,0x00, // '.'
    0x20,0x10,0x08,0x04,0x02, // '/'
    0x3E,0x51,0x49,0x45,0x3E, // '0'
    0x00,0x42,0x7F,0x40,0x00, // '1'
    0x42,0x61,0x51,0x49,0x46, // '2'
    0x21,0x41,0x45,0x4B,0x31, // '3'
    0x18,0x14,0x12,0x7F,0x10, // '4'
    0x27,0x45,0x45,0x45,0x39, // '5'
    0x3C,0x4A,0x49,0x49,0x30, // '6'
    0x01,0x71,0x09,0x05,0x03, // '7'
    0x36,0x49,0x49,0x49,0x36, // '8'
    0x06,0x49,0x49,0x29,0x1E, // '9'
    0x00,0x36,0x36,0x00,0x00, // ':'
    0x00,0x56,0x36,0x00,0x00, // ';'
    0x08,0x14,0x22,0x41,0x00, // '<'
    0x14,0x14,0x14,0x14,0x14, // '='
    0x00,0x41,0x22,0x14,0x08, // '>'
    0x02,0x01,0x51,0x09,0x06, // '?'
    0x32,0x49,0x79,0x41,0x3E, // '@'
    0x7E,0x11,0x11,0x11,0x7E, // 'A'
    0x7F,0x49,0x49,0x49,0x36, // 'B'
    0x3E,0x41,0x41,0x41,0x22, // 'C'
    0x7F,0x41,0x41,0x22,0x1C, // 'D'
    0x7F,0x49,0x49,0x49,0x41, // 'E'
    0x7F,0x09,0x09,0x09,0x01, // 'F'
    0x3E,0x41,0x49,0x49,0x7A, // 'G'
    0x7F,0x08,0x08,0x08,0x7F, // 'H'
    0x00,0x41,0x7F,0x41,0x00, // 'I'
    0x20,0x40,0x41,0x3F,0x01, // 'J'
    0x7F,0x08,0x14,0x22,0x41, // 'K'
    0x7F,0x40,0x40,0x40,0x40, // 'L'
    0x7F,0x02,0x0C,0x02,0x7F, // 'M'
    0x7F,0x04,0x08,0x10,0x7F, // 'N'
    0x3E,0x41,0x41,0x41,0x3E, // 'O'
    0x7F,0x09,0x09,0x09,0x06, // 'P'
    0x3E,0x41,0x51,0x21,0x5E, // 'Q'
    0x7F,0x09,0x19,0x29,0x46, // 'R'
    0x46,0x49,0x49,0x49,0x31, // 'S'
    0x01,0x01,0x7F,0x01,0x01, // 'T'
    0x3F,0x40,0x40,0x40,0x3F, // 'U'
    0x1F,0x20,0x40,0x20,0x1F, // 'V'
    0x3F,0x40,0x38,0x40,0x3F, // 'W'
    0x63,0x14,0x08,0x14,0x63, // 'X'
    0x07,0x08,0x70,0x08,0x07, // 'Y'
    0x61,0x51,0x49,0x45,0x43, // 'Z'
    0x00,0x7F,0x41,0x41,0x00, // '['
    0x02,0x04,0x08,0x10,0x20, // '\'
    0x00,0x41,0x41,0x7F,0x00, // ']'
    0x04,0x02,0x01,0x02,0x04, // '^'
    0x40,0x40,0x40,0x40,0x40, // '_'
    0x00,0x01,0x02,0x04,0x00, // '`'
    0x20,0x54,0x54,0x54,0x78, // 'a'
    0x7F,0x48,0x44,0x44,0x38, // 'b'
    0x38,0x44,0x44,0x44,0x20, // 'c'
    0x38,0x44,0x44,0x48,0x7F, // 'd'
    0x38,0x54,0x54,0x54,0x18, // 'e'
    0x08,0x7E,0x09,0x01,0x02, // 'f'
    0x0C,0x52,0x52,0x52,0x3E, // 'g'
    0x7F,0x08,0x04,0x04,0x78, // 'h'
    0x00,0x44,0x7D,0x40,0x00, // 'i'
    0x20,0x40,0x44,0x3D,0x00, // 'j'
    0x7F,0x10,0x28,0x44,0x00, // 'k'
    0x00,0x41,0x7F,0x40,0x00, // 'l'
    0x7C,0x04,0x18,0x04,0x78, // 'm'
    0x7C,0x08,0x04,0x04,0x78, // 'n'
    0x38,0x44,0x44,0x44,0x38, // 'o'
    0x7C,0x14,0x14,0x14,0x08, // 'p'
    0x08,0x14,0x14,0x18,0x7C, // 'q'
    0x7C,0x08,0x04,0x04,0x08, // 'r'
    0x48,0x54,0x54,0x54,0x20, // 's'
    0x04,0x3F,0x44,0x40,0x20, // 't'
    0x3C,0x40,0x40,0x20,0x7C, // 'u'
    0x1C,0x20,0x40,0x20,0x1C, // 'v'
    0x3C,0x40,0x30,0x40,0x3C, // 'w'
    0x44,0x28,0x10,0x28,0x44, // 'x'
    0x0C,0x50,0x50,0x50,0x3C, // 'y'
    0x44,0x64,0x54,0x4C,0x44, // 'z'
    0x00,0x08,0x36,0x41,0x00, // '{'
    0x00,0x00,0x7F,0x00,0x00, // '|'
    0x00,0x41,0x36,0x08,0x00, // '}'
    0x08,0x08,0x2A,0x1C,0x08, // '~'
];

pub(crate) const FONT_CHAR_W: i32 = 6;
pub(crate) const FONT_CHAR_H: i32 = 8;

fn draw_glyph(fb: &mut [u32], fb_w: i32, fb_h: i32, x: i32, y: i32, ch: char, col: Colour) {
    let code = ch as u32;
    if code < 0x20 || code > 0x7E { return; }
    let idx = ((code - 0x20) as usize) * 5;
    let bytes = &FONT_5X7[idx..idx + 5];
    for col_x in 0..5i32 {
        let byte = bytes[col_x as usize];
        for row_y in 0..7i32 {
            if byte & (1 << row_y) != 0 {
                let px = x + col_x;
                let py = y + row_y;
                if px >= 0 && px < fb_w && py >= 0 && py < fb_h {
                    fb[(py * fb_w + px) as usize] = col.to_argb_u32();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Framebuffer
// ---------------------------------------------------------------------------

pub struct Framebuffer {
    pub width:  usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            pixels: vec![0xFF000000u32; width * height],
        }
    }

    #[inline]
    fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }

    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, col: Colour) {
        if self.in_bounds(x, y) {
            let idx = y as usize * self.width + x as usize;
            let bg = Colour::from_argb_u32(self.pixels[idx]);
            self.pixels[idx] = col.blend_over(bg).to_argb_u32();
        }
    }

    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> Colour {
        if self.in_bounds(x, y) {
            Colour::from_argb_u32(self.pixels[y as usize * self.width + x as usize])
        } else {
            Colour::BLACK
        }
    }

    pub fn clear(&mut self, col: Colour) {
        self.pixels.fill(col.to_argb_u32());
    }

    pub fn draw_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, col: Colour) {
        let (mut x, mut y) = (x0, y0);
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx: i32 = if x0 < x1 { 1 } else { -1 };
        let sy: i32 = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.set_pixel(x, y, col);
            if x == x1 && y == y1 { break; }
            let e2 = 2 * err;
            if e2 >= dy { if x == x1 { break; } err += dy; x += sx; }
            if e2 <= dx { if y == y1 { break; } err += dx; y += sy; }
        }
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, w: i32, h: i32, col: Colour) {
        self.draw_line(x,     y,     x+w-1, y,     col);
        self.draw_line(x,     y+h-1, x+w-1, y+h-1, col);
        self.draw_line(x,     y,     x,     y+h-1, col);
        self.draw_line(x+w-1, y,     x+w-1, y+h-1, col);
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, col: Colour) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.width as i32);
        let y1 = (y + h).min(self.height as i32);
        let fill = col.to_argb_u32();
        for py in y0..y1 {
            let row_start = py as usize * self.width;
            if col.a == 255 {
                for px in x0..x1 {
                    self.pixels[row_start + px as usize] = fill;
                }
            } else {
                for px in x0..x1 {
                    let idx = row_start + px as usize;
                    let existing = Colour::from_argb_u32(self.pixels[idx]);
                    self.pixels[idx] = col.blend_over(existing).to_argb_u32();
                }
            }
        }
    }

    pub fn draw_circle(&mut self, cx: i32, cy: i32, r: i32, col: Colour) {
        if r <= 0 { self.set_pixel(cx, cy, col); return; }
        let (mut x, mut y) = (0i32, r);
        let mut d = 3 - 2 * r;
        self.plot_circle_8(cx, cy, x, y, col);
        while y >= x {
            x += 1;
            if d > 0 { y -= 1; d += 4 * (x - y) + 10; }
            else      { d += 4 * x + 6; }
            self.plot_circle_8(cx, cy, x, y, col);
        }
    }

    fn plot_circle_8(&mut self, cx: i32, cy: i32, x: i32, y: i32, col: Colour) {
        let pts = [
            (cx+x, cy+y), (cx-x, cy+y), (cx+x, cy-y), (cx-x, cy-y),
            (cx+y, cy+x), (cx-y, cy+x), (cx+y, cy-x), (cx-y, cy-x),
        ];
        for (px, py) in pts { self.set_pixel(px, py, col); }
    }

    pub fn fill_circle(&mut self, cx: i32, cy: i32, r: i32, col: Colour) {
        if r <= 0 { self.set_pixel(cx, cy, col); return; }
        let (mut x, mut y) = (0i32, r);
        let mut d = 3 - 2 * r;
        self.fill_circle_hline(cx, cy, x, y, col);
        while y >= x {
            x += 1;
            if d > 0 { y -= 1; d += 4 * (x - y) + 10; }
            else      { d += 4 * x + 6; }
            self.fill_circle_hline(cx, cy, x, y, col);
        }
    }

    fn fill_circle_hline(&mut self, cx: i32, cy: i32, x: i32, y: i32, col: Colour) {
        self.fill_rect(cx - x, cy + y, 2 * x, 1, col);
        self.fill_rect(cx - x, cy - y, 2 * x, 1, col);
        self.fill_rect(cx - y, cy + x, 2 * y, 1, col);
        self.fill_rect(cx - y, cy - x, 2 * y, 1, col);
    }

    pub fn draw_triangle(
        &mut self,
        x0: i32, y0: i32,
        x1: i32, y1: i32,
        x2: i32, y2: i32,
        col: Colour,
    ) {
        self.draw_line(x0, y0, x1, y1, col);
        self.draw_line(x1, y1, x2, y2, col);
        self.draw_line(x2, y2, x0, y0, col);
    }

    pub fn fill_triangle(
        &mut self,
        x0: i32, y0: i32,
        x1: i32, y1: i32,
        x2: i32, y2: i32,
        col: Colour,
    ) {
        let mut pts = [(x0,y0),(x1,y1),(x2,y2)];
        pts.sort_by_key(|p| p.1);
        let (ax,ay) = pts[0];
        let (bx,by) = pts[1];
        let (cx,cy) = pts[2];
        let total_h = cy - ay;
        if total_h == 0 { return; }
        for y in ay..=cy {
            let second_half = y >= by;
            let seg_h = if second_half { cy - by } else { by - ay };
            let seg_h = if seg_h == 0 { 1 } else { seg_h };
            let alpha = (y - ay) as f32 / total_h as f32;
            let beta  = if second_half {
                (y - by) as f32 / seg_h as f32
            } else {
                (y - ay) as f32 / seg_h as f32
            };
            let mut xa = ax as f32 + (cx - ax) as f32 * alpha;
            let mut xb = if second_half {
                bx as f32 + (cx - bx) as f32 * beta
            } else {
                ax as f32 + (bx - ax) as f32 * beta
            };
            if xa > xb { core::mem::swap(&mut xa, &mut xb); }
            for x in (xa as i32)..=(xb as i32) {
                self.set_pixel(x, y, col);
            }
        }
    }

    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, col: Colour) {
        let w = self.width as i32;
        let h = self.height as i32;
        let mut cur_x = x;
        for ch in text.chars() {
            if ch == '\n' { continue; }
            draw_glyph(&mut self.pixels, w, h, cur_x, y, ch, col);
            cur_x += FONT_CHAR_W;
        }
    }

    /// Teks yang diperbesar, tiap piksel font jadi kotak `skala x skala`.
    pub fn draw_text_scaled(&mut self, x: i32, y: i32, text: &str, skala: i32, col: Colour) {
        let skala = skala.max(1);
        let mut cur_x = x;
        for ch in text.chars() {
            if ch == '\n' { continue; }
            let code = ch as u32;
            if (0x20..=0x7E).contains(&code) {
                let idx = ((code - 0x20) as usize) * 5;
                let bytes: [u8; 5] = [
                    FONT_5X7[idx], FONT_5X7[idx + 1], FONT_5X7[idx + 2],
                    FONT_5X7[idx + 3], FONT_5X7[idx + 4],
                ];
                for cx in 0..5i32 {
                    for ry in 0..7i32 {
                        if bytes[cx as usize] & (1 << ry) != 0 {
                            self.fill_rect(cur_x + cx * skala, y + ry * skala, skala, skala, col);
                        }
                    }
                }
            }
            cur_x += FONT_CHAR_W * skala;
        }
    }

    pub fn text_width(text: &str) -> i32 {
        text.len() as i32 * FONT_CHAR_W
    }

    /// Isi berkas PPM (P6) bingkai ini.
    pub fn ppm(&self) -> Vec<u8> {
        let kepala = alloc::format!("P6\n{} {}\n255\n", self.width, self.height);
        let mut data = Vec::with_capacity(kepala.len() + self.width * self.height * 3);
        data.extend_from_slice(kepala.as_bytes());
        for &px in &self.pixels {
            data.push(((px >> 16) & 0xFF) as u8);
            data.push(((px >>  8) & 0xFF) as u8);
            data.push((px         & 0xFF) as u8);
        }
        data
    }

    /// Isi berkas BMP 24 bit (baris dari atas) bingkai ini.
    pub fn bmp(&self) -> Vec<u8> {
        let row_size = ((self.width * 3 + 3) / 4) * 4;
        let pixel_data_size = row_size * self.height;
        let file_size = 54 + pixel_data_size;
        let mut f = Vec::with_capacity(file_size);
        f.extend_from_slice(b"BM");
        f.extend_from_slice(&(file_size as u32).to_le_bytes());
        f.extend_from_slice(&0u32.to_le_bytes());
        f.extend_from_slice(&54u32.to_le_bytes());
        f.extend_from_slice(&40u32.to_le_bytes());
        f.extend_from_slice(&(self.width as i32).to_le_bytes());
        f.extend_from_slice(&(-(self.height as i32)).to_le_bytes());
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&24u16.to_le_bytes());
        f.extend_from_slice(&0u32.to_le_bytes());
        f.extend_from_slice(&(pixel_data_size as u32).to_le_bytes());
        f.extend_from_slice(&2835i32.to_le_bytes());
        f.extend_from_slice(&2835i32.to_le_bytes());
        f.extend_from_slice(&0u32.to_le_bytes());
        f.extend_from_slice(&0u32.to_le_bytes());
        let mut row_buf = vec![0u8; row_size];
        for y in 0..self.height {
            for x in 0..self.width {
                let px = self.pixels[y * self.width + x];
                let i = x * 3;
                row_buf[i]   = (px         & 0xFF) as u8;
                row_buf[i+1] = ((px >>  8) & 0xFF) as u8;
                row_buf[i+2] = ((px >> 16) & 0xFF) as u8;
            }
            for i in (self.width * 3)..row_size { row_buf[i] = 0; }
            f.extend_from_slice(&row_buf);
        }
        f
    }
}

#[derive(Debug, Default, Clone)]
pub struct KeyState {
    pub pressed:  BTreeSet<String>,
    pub held:     BTreeSet<String>,
}

impl KeyState {
    pub fn is_pressed(&self, key: &str) -> bool { self.pressed.contains(key) }
    pub fn is_held(&self,    key: &str) -> bool { self.held.contains(key) }
}
