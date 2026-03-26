pub fn current_page_indices(filtered: &[usize], page: usize, page_size: usize) -> &[usize] {
    if filtered.is_empty() {
        return &filtered[0..0];
    }

    let start = (page * page_size).min(filtered.len());
    let end = (start + page_size).min(filtered.len());
    &filtered[start..end]
}

pub fn page_count(filtered_len: usize, page_size: usize) -> usize {
    filtered_len.div_ceil(page_size)
}

pub fn compact_preview_text(
    preview: &str,
    preview_line_limit: usize,
    preview_char_limit: usize,
) -> String {
    let lines: Vec<_> = preview
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    let truncated_by_lines = lines.len() > preview_line_limit;
    let mut compact = if lines.is_empty() {
        preview.trim().to_string()
    } else {
        lines
            .into_iter()
            .take(preview_line_limit)
            .collect::<Vec<_>>()
            .join("\n")
    };

    if compact.is_empty() {
        return "(empty entry)".to_string();
    }

    let truncated_by_chars = compact.chars().count() > preview_char_limit;
    if truncated_by_chars {
        compact = compact.chars().take(preview_char_limit).collect();
        compact = compact.trim_end().to_string();
    }

    if truncated_by_lines || truncated_by_chars {
        compact.push_str("...");
    }

    compact
}

pub fn next_selected_index(
    filtered: &[usize],
    current: Option<usize>,
    delta: i32,
) -> Option<usize> {
    if filtered.is_empty() {
        return None;
    }

    if delta == i32::MIN {
        return filtered.first().copied();
    }

    if delta == i32::MAX {
        return filtered.last().copied();
    }

    if current.is_none() {
        return if delta < 0 {
            filtered.last().copied()
        } else {
            filtered.first().copied()
        };
    }

    let position = current
        .and_then(|index| filtered.iter().position(|candidate| *candidate == index))
        .unwrap_or(0) as i32;
    let next = (position + delta).clamp(0, filtered.len() as i32 - 1) as usize;
    filtered.get(next).copied()
}

pub fn looks_like_html(preview: &str) -> bool {
    let preview = preview.trim_start().to_ascii_lowercase();

    preview.starts_with("<!doctype html")
        || preview.starts_with("<html")
        || preview.starts_with("<meta http-equiv=\"content-type\" content=\"text/html")
        || preview.starts_with("<img ")
        || preview.contains("<body")
        || preview.contains("</html>")
}

pub fn stderr_message(prefix: &str, stderr: &str) -> String {
    if stderr.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}: {stderr}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn selection_moves_inside_filtered_results() {
        let filtered = vec![2, 5, 9];
        assert_eq!(next_selected_index(&filtered, None, 1), Some(2));
        assert_eq!(next_selected_index(&filtered, None, -1), Some(9));
        assert_eq!(next_selected_index(&filtered, Some(5), 1), Some(9));
        assert_eq!(next_selected_index(&filtered, Some(9), 1), Some(9));
        assert_eq!(next_selected_index(&filtered, Some(9), -10), Some(2));
        assert_eq!(next_selected_index(&filtered, Some(5), i32::MIN), Some(2));
        assert_eq!(next_selected_index(&filtered, Some(5), i32::MAX), Some(9));
    }

    #[test]
    fn paging_slices_the_filtered_results() {
        let cfg = Config::default();
        let filtered: Vec<_> = (0..45).collect();
        assert_eq!(page_count(filtered.len(), cfg.page_size), 3);
        assert_eq!(current_page_indices(&filtered, 0, cfg.page_size).len(), 16);
        assert_eq!(current_page_indices(&filtered, 1, cfg.page_size).len(), 16);
        assert_eq!(current_page_indices(&filtered, 2, cfg.page_size).len(), 13);
    }

    #[test]
    fn paging_handles_empty_and_out_of_range_pages() {
        let cfg = Config::default();
        let empty: Vec<usize> = Vec::new();
        let filtered: Vec<_> = (0..5).collect();

        assert_eq!(page_count(empty.len(), cfg.page_size), 0);
        assert!(current_page_indices(&empty, 0, cfg.page_size).is_empty());
        assert!(current_page_indices(&filtered, 10, cfg.page_size).is_empty());
    }

    #[test]
    fn compact_preview_limits_large_entries() {
        let cfg = Config::default();
        let preview = "line 1\nline 2\nline 3\nline 4\nline 5";
        assert_eq!(
            compact_preview_text(preview, cfg.preview_line_limit, cfg.preview_char_limit),
            "line 1\nline 2\nline 3\nline 4..."
        );
    }

    #[test]
    fn compact_preview_handles_empty_and_char_truncation() {
        let cfg = Config::default();
        assert_eq!(
            compact_preview_text("   \n  ", cfg.preview_line_limit, cfg.preview_char_limit),
            "(empty entry)"
        );

        let long = "a".repeat(cfg.preview_char_limit + 10);
        let compact = compact_preview_text(&long, cfg.preview_line_limit, cfg.preview_char_limit);

        assert_eq!(compact.len(), cfg.preview_char_limit + 3);
        assert!(compact.ends_with("..."));
    }

    #[test]
    fn html_detection_and_stderr_helpers_cover_edge_cases() {
        assert!(looks_like_html("<body>hello</body>"));
        assert!(looks_like_html("<img src=\"x\">"));
        assert!(!looks_like_html("plain text"));

        assert_eq!(stderr_message("prefix", ""), "prefix");
        assert_eq!(stderr_message("prefix", "stderr"), "prefix: stderr");
    }
}
