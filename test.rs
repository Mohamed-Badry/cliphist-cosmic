use cosmic::app::Settings;
use cosmic::iced::window;
fn main() {
    let s = Settings::default().window(window::Settings {
        level: window::Level::AlwaysOnTop,
        ..Default::default()
    });
}
