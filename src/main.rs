mod app;
mod cliphist;
mod config;
mod models;
mod utils;

use cosmic::app::Settings;
use cosmic::iced::Limits;
use cosmic::iced::Size;

use config::{WINDOW_HEIGHT, WINDOW_WIDTH};

fn main() -> cosmic::iced::Result {
    let settings = Settings::default()
        .no_main_window(true)
        .size(Size::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .size_limits(
            Limits::NONE
                .min_width(WINDOW_WIDTH)
                .max_width(WINDOW_WIDTH)
                .min_height(WINDOW_HEIGHT)
                .max_height(WINDOW_HEIGHT),
        )
        .resizable(None)
        .client_decorations(false)
        .transparent(false);

    cosmic::app::run::<app::ClipboardApp>(settings, ())
}
