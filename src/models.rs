use crate::utils::{compact_preview_text, looks_like_html};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipKind {
    Text,
    Html,
    Binary(String),
}

impl ClipKind {
    pub fn from_preview(preview: &str) -> Self {
        if let Some(mime) = preview.strip_prefix("binary data ") {
            return Self::Binary(mime.trim().to_string());
        }

        if looks_like_html(preview) {
            return Self::Html;
        }

        Self::Text
    }

    pub fn mime_type(&self) -> Option<&str> {
        match self {
            Self::Text => None,
            Self::Html => Some("text/html"),
            Self::Binary(mime) => Some(mime.as_str()),
        }
    }

    pub fn is_image(&self) -> bool {
        matches!(self, Self::Binary(mime) if mime.starts_with("image/"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipItem {
    pub line: String,
    pub id: String,
    pub preview: String,
    pub preview_lower: String,
    pub kind: ClipKind,
}

impl ClipItem {
    pub fn parse(line: &str) -> Option<Self> {
        let (id, preview) = line.split_once('\t')?;
        let preview = preview.to_string();

        Some(Self {
            line: line.to_string(),
            id: id.to_string(),
            preview_lower: preview.to_lowercase(),
            kind: ClipKind::from_preview(&preview),
            preview,
        })
    }

    pub fn preview_text(&self, preview_line_limit: usize, preview_char_limit: usize) -> String {
        compact_preview_text(&self.preview, preview_line_limit, preview_char_limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn parses_cliphist_list_lines() {
        let item = ClipItem::parse("42\tbinary data image/png").expect("expected parsed item");
        assert_eq!(item.id, "42");
        assert_eq!(item.preview, "binary data image/png");
        assert_eq!(item.kind, ClipKind::Binary("image/png".to_string()));
    }

    #[test]
    fn detects_html_preview() {
        assert_eq!(
            ClipKind::from_preview(
                "<meta http-equiv=\"content-type\" content=\"text/html; charset=utf-8\">"
            ),
            ClipKind::Html
        );
        assert_eq!(ClipKind::from_preview("plain text"), ClipKind::Text);
    }

    #[test]
    fn mime_types_and_image_detection_match_kind() {
        assert_eq!(ClipKind::Text.mime_type(), None);
        assert_eq!(ClipKind::Html.mime_type(), Some("text/html"));
        assert_eq!(
            ClipKind::Binary("image/png".to_string()).mime_type(),
            Some("image/png")
        );
        assert!(ClipKind::Binary("image/png".to_string()).is_image());
        assert!(!ClipKind::Binary("application/pdf".to_string()).is_image());
    }

    #[test]
    fn parse_rejects_invalid_lines_and_preview_text_is_compacted() {
        let cfg = Config::default();
        assert_eq!(ClipItem::parse("missing-tab"), None);

        let item = ClipItem::parse("7\tline 1\nline 2\nline 3\nline 4\nline 5")
            .expect("expected parsed item");
        assert_eq!(item.id, "7");
        assert!(
            item.preview_text(cfg.preview_line_limit, cfg.preview_char_limit)
                .ends_with("...")
        );
    }
}
