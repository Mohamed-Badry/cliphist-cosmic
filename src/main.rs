mod app;
mod cliphist;
mod config;
mod keyboard;
mod messages;
mod models;
mod utils;
mod view;
mod vim;

use clap::Parser;
use cosmic::app::Settings;
use cosmic::iced::Limits;
use cosmic::iced::Size;

use config::Config;

#[derive(Parser, Debug)]
#[command(name = "cliphist-cosmic", about = "A Wayland clipboard manager")]
pub struct Cli {
    #[arg(long, help = "Enable Vim keybindings")]
    pub vim: bool,

    #[arg(long, help = "Window width in pixels", default_value_t = Config::default().window_width)]
    pub width: f32,

    #[arg(long, help = "Window height in pixels", default_value_t = Config::default().window_height)]
    pub height: f32,

    #[arg(long, help = "Number of items per page", default_value_t = Config::default().page_size)]
    pub page_size: usize,

    #[arg(long, help = "Image preview height in pixels", default_value_t = Config::default().image_height)]
    pub image_height: f32,

    #[arg(long, help = "Max preview lines for text entries", default_value_t = Config::default().preview_line_limit)]
    pub preview_lines: usize,

    #[arg(long, help = "Max preview characters for text entries", default_value_t = Config::default().preview_char_limit)]
    pub preview_chars: usize,
}

fn main() -> cosmic::iced::Result {
    let cli = Cli::parse();

    let config = Config {
        window_width: cli.width,
        window_height: cli.height,
        page_size: cli.page_size,
        image_height: cli.image_height,
        preview_line_limit: cli.preview_lines,
        preview_char_limit: cli.preview_chars,
    };

    let settings = Settings::default()
        .size(Size::new(config.window_width, config.window_height))
        .size_limits(
            Limits::NONE
                .min_width(config.window_width)
                .max_width(config.window_width)
                .min_height(config.window_height)
                .max_height(config.window_height),
        )
        .resizable(None)
        .client_decorations(false)
        .transparent(false);

    cosmic::app::run::<app::ClipboardApp>(settings, (cli.vim, config))
}
