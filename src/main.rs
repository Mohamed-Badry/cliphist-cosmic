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

use clap::{CommandFactory, Parser};
use cosmic::app::Settings;
use cosmic::iced::Limits;
use cosmic::iced::Size;

use config::{Config, LayerPlacement, PositionPreset, SurfaceMode};

#[derive(Parser, Debug)]
#[command(name = "cliphist-cosmic", about = "A Wayland clipboard manager")]
pub struct Cli {
    #[arg(long, help = "Enable Vim keybindings")]
    pub vim: bool,

    #[arg(
        long,
        value_enum,
        default_value_t = SurfaceMode::Window,
        help = "Startup surface mode: window keeps mouse drag, layer enables fixed placement"
    )]
    pub surface: SurfaceMode,

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

    #[arg(long, value_enum, help = "Layer-surface preset position")]
    pub position: Option<PositionPreset>,

    #[arg(
        long,
        requires = "y",
        help = "Layer-surface absolute X coordinate in pixels"
    )]
    pub x: Option<i32>,

    #[arg(
        long,
        requires = "x",
        help = "Layer-surface absolute Y coordinate in pixels"
    )]
    pub y: Option<i32>,
}

impl Cli {
    fn validate(&self) -> Result<(), clap::Error> {
        if self.surface == SurfaceMode::Window && (self.position.is_some() || self.x.is_some()) {
            return Err(Self::command().error(
                clap::error::ErrorKind::ArgumentConflict,
                "--position, --x, and --y require --surface layer",
            ));
        }

        Ok(())
    }
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
    if let Err(err) = cli.validate() {
        err.exit();
    }

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
        surface_mode: cli.surface,
        layer_placement: LayerPlacement::new(cli.position, cli.x, cli.y),
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
        .transparent(false)
        .no_main_window(config.uses_layer_surface());

    cosmic::app::run::<app::ClipboardApp>(settings, (cli.vim, config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_defaults_to_normal_window_mode() {
        let cli = Cli::try_parse_from(["cliphist-cosmic"]).expect("cli should parse");
        assert_eq!(cli.surface, SurfaceMode::Window);
        assert_eq!(cli.position, None);
        assert_eq!(cli.x, None);
        assert_eq!(cli.y, None);
    }

    #[test]
    fn cli_accepts_layer_presets() {
        let cli = Cli::try_parse_from([
            "cliphist-cosmic",
            "--surface",
            "layer",
            "--position",
            "top-right",
        ])
        .expect("cli should parse");

        assert_eq!(cli.surface, SurfaceMode::Layer);
        assert_eq!(cli.position, Some(PositionPreset::TopRight));
        assert!(cli.validate().is_ok());
    }

    #[test]
    fn cli_requires_coordinate_pairs() {
        assert!(Cli::try_parse_from(["cliphist-cosmic", "--x", "10"]).is_err());
        assert!(Cli::try_parse_from(["cliphist-cosmic", "--y", "10"]).is_err());
    }

    #[test]
    fn cli_rejects_placement_without_layer_mode() {
        let cli = Cli::try_parse_from(["cliphist-cosmic", "--position", "center"])
            .expect("cli should parse before validation");

        assert!(cli.validate().is_err());
    }
}
