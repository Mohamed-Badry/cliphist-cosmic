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

use config::{WINDOW_HEIGHT, WINDOW_WIDTH};

#[derive(Parser, Debug)]
#[command(name = "cliprs", about = "A Wayland clipboard manager")]
pub struct Cli {
    #[arg(long, help = "Enable Vim keybindings")]
    pub vim: bool,
}

fn main() -> cosmic::iced::Result {
    let cli = Cli::parse();
    let settings = Settings::default()
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

    cosmic::app::run::<app::ClipboardApp>(settings, cli.vim)
}
