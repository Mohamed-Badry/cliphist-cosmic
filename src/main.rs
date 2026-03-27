mod app;
mod cliphist;
mod config;
mod keyboard;
mod messages;
mod models;
mod utils;
mod view;
mod vim;

use std::fs;
use std::io::Write;
use std::path::PathBuf;

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

    #[arg(long, help = "Disable toggle behavior (always start a new instance)")]
    pub no_toggle: bool,
}

fn pid_file_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(runtime_dir).join("cliphist-cosmic.pid")
}

fn try_toggle() -> bool {
    let path = pid_file_path();

    if let Ok(contents) = fs::read_to_string(&path) {
        if let Ok(pid) = contents.trim().parse::<i32>() {
            // Check if process is alive
            let alive = unsafe { libc::kill(pid, 0) } == 0;
            if alive {
                // Send SIGTERM to close the running instance
                unsafe { libc::kill(pid, libc::SIGTERM) };
                return true;
            }
        }
        // Stale PID file — remove it and continue
        let _ = fs::remove_file(&path);
    }

    false
}

fn write_pid_file() {
    let path = pid_file_path();
    if let Ok(mut file) = fs::File::create(&path) {
        let _ = write!(file, "{}", std::process::id());
    }
}

fn cleanup_pid_file() {
    let _ = fs::remove_file(pid_file_path());
}

struct PidGuard;

impl Drop for PidGuard {
    fn drop(&mut self) {
        cleanup_pid_file();
    }
}

fn main() -> cosmic::iced::Result {
    let cli = Cli::parse();

    // Toggle logic: if another instance is running, close it and exit
    if !cli.no_toggle && try_toggle() {
        return Ok(());
    }

    // Write PID file and register cleanup
    write_pid_file();
    let _pid_guard = PidGuard;

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
