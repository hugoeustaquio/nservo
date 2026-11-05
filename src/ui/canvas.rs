use crate::ui::font::{get_glyph_bitmap, FONT_HEIGHT, FONT_WIDTH};

/// In-memory 2D graphical pixel buffer for software and GPU frame presentation.
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub buffer: Vec<u32>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            buffer: vec![0; width * height],
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.buffer.resize(width * height, 0);
    }

    pub fn clear(&mut self, color: u32) {
        self.buffer.fill(color);
    }

    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = color;
        }
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        let max_x = (x + w).min(self.width);
        let max_y = (y + h).min(self.height);

        for row in y..max_y {
            let start = row * self.width + x;
            let end = row * self.width + max_x;
            if start < self.buffer.len() && end <= self.buffer.len() {
                self.buffer[start..end].fill(color);
            }
        }
    }

    pub fn draw_char(&mut self, x: usize, y: usize, c: char, fg: u32, bg: Option<u32>) {
        let bitmap = get_glyph_bitmap(c);
        for row in 0..FONT_HEIGHT {
            let row_byte = bitmap[row];
            for col in 0..FONT_WIDTH {
                let is_set = (row_byte & (0x80 >> col)) != 0;
                let px = x + col;
                let py = y + row;
                if is_set {
                    self.set_pixel(px, py, fg);
                } else if let Some(bg_color) = bg {
                    self.set_pixel(px, py, bg_color);
                }
            }
        }
    }

    pub fn draw_text(&mut self, x: usize, y: usize, text: &str, fg: u32, bg: Option<u32>) {
        let mut cur_x = x;
        for c in text.chars() {
            if cur_x + FONT_WIDTH > self.width {
                break;
            }
            self.draw_char(cur_x, y, c, fg, bg);
            cur_x += FONT_WIDTH;
        }
    }
}
