use clap::ValueEnum;
use cosmic::app::Task;
use cosmic::iced::Limits;
use cosmic::iced::platform_specific::runtime::wayland::layer_surface::{
    IcedMargin, SctkLayerSurfaceSettings,
};
use cosmic::iced::platform_specific::shell::commands::layer_surface::{
    Anchor, KeyboardInteractivity, Layer, get_layer_surface,
};
use cosmic::iced::window;

const APP_NAMESPACE: &str = "com.github.cliphist_cosmic";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum SurfaceMode {
    #[default]
    Window,
    Layer,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum PositionPreset {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    #[default]
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayerPlacement {
    pub position: PositionPreset,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

impl LayerPlacement {
    pub fn new(position: Option<PositionPreset>, x: Option<i32>, y: Option<i32>) -> Self {
        Self {
            position: position.unwrap_or_default(),
            x,
            y,
        }
    }

    pub fn has_absolute_coordinates(self) -> bool {
        self.x.is_some() && self.y.is_some()
    }

    fn anchor(self) -> Anchor {
        if self.has_absolute_coordinates() {
            return Anchor::TOP | Anchor::LEFT;
        }

        match self.position {
            PositionPreset::TopLeft => Anchor::TOP | Anchor::LEFT,
            PositionPreset::TopCenter => Anchor::TOP,
            PositionPreset::TopRight => Anchor::TOP | Anchor::RIGHT,
            PositionPreset::CenterLeft => Anchor::LEFT,
            PositionPreset::Center => Anchor::empty(),
            PositionPreset::CenterRight => Anchor::RIGHT,
            PositionPreset::BottomLeft => Anchor::BOTTOM | Anchor::LEFT,
            PositionPreset::BottomCenter => Anchor::BOTTOM,
            PositionPreset::BottomRight => Anchor::BOTTOM | Anchor::RIGHT,
        }
    }

    fn margin(self) -> IcedMargin {
        match (self.x, self.y) {
            (Some(x), Some(y)) => IcedMargin {
                top: y,
                left: x,
                ..IcedMargin::default()
            },
            _ => IcedMargin::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub window_width: f32,
    pub window_height: f32,
    pub page_size: usize,
    pub image_height: f32,
    pub preview_line_limit: usize,
    pub preview_char_limit: usize,
    pub surface_mode: SurfaceMode,
    pub layer_placement: LayerPlacement,
}

impl Config {
    pub fn uses_layer_surface(self) -> bool {
        self.surface_mode == SurfaceMode::Layer
    }

    pub fn allows_mouse_drag(self) -> bool {
        self.surface_mode == SurfaceMode::Window
    }

    pub fn layer_surface_task<Message: 'static>(self) -> Task<Message> {
        if !self.uses_layer_surface() {
            return Task::none();
        }

        let width = dimension_to_u32(self.window_width);
        let height = dimension_to_u32(self.window_height);

        get_layer_surface::<cosmic::Action<Message>>(SctkLayerSurfaceSettings {
            id: window::Id::unique(),
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            anchor: self.layer_placement.anchor(),
            namespace: APP_NAMESPACE.to_string(),
            margin: self.layer_placement.margin(),
            size: Some((Some(width), Some(height))),
            exclusive_zone: 0,
            size_limits: Limits::NONE
                .min_width(self.window_width)
                .max_width(self.window_width)
                .min_height(self.window_height)
                .max_height(self.window_height),
            ..SctkLayerSurfaceSettings::default()
        })
    }
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
            surface_mode: SurfaceMode::default(),
            layer_placement: LayerPlacement::default(),
        }
    }
}

fn dimension_to_u32(value: f32) -> u32 {
    value.max(1.0).round().min(u32::MAX as f32) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_anchor_maps_to_expected_edges() {
        assert_eq!(
            LayerPlacement::new(Some(PositionPreset::TopRight), None, None).anchor(),
            Anchor::TOP | Anchor::RIGHT
        );
        assert_eq!(
            LayerPlacement::new(Some(PositionPreset::Center), None, None).anchor(),
            Anchor::empty()
        );
        assert_eq!(
            LayerPlacement::new(Some(PositionPreset::BottomLeft), None, None).anchor(),
            Anchor::BOTTOM | Anchor::LEFT
        );
    }

    #[test]
    fn absolute_coordinates_override_presets() {
        let placement = LayerPlacement::new(Some(PositionPreset::BottomRight), Some(24), Some(48));
        let margin = placement.margin();

        assert_eq!(placement.anchor(), Anchor::TOP | Anchor::LEFT);
        assert_eq!(margin.top, 48);
        assert_eq!(margin.left, 24);
        assert_eq!(margin.right, 0);
        assert_eq!(margin.bottom, 0);
    }

    #[test]
    fn window_mode_keeps_drag_enabled() {
        let config = Config::default();
        assert!(!config.uses_layer_surface());
        assert!(config.allows_mouse_drag());
    }
}
