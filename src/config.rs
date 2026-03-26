#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub window_width: f32,
    pub window_height: f32,
    pub page_size: usize,
    pub image_height: f32,
    pub preview_line_limit: usize,
    pub preview_char_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_width: 480.0,
            window_height: 560.0,
            page_size: 16,
            image_height: 116.0,
            preview_line_limit: 4,
            preview_char_limit: 280,
        }
    }
}
