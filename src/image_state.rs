use std::collections::{HashMap, VecDeque};

use cosmic::widget;

use crate::cliphist::ImagePreview;

#[derive(Debug)]
pub struct ImageState {
    page_handles: HashMap<usize, widget::image::Handle>,
    page_errors: HashMap<usize, String>,
    cache: PreviewCache,
    request_id: u64,
}

impl ImageState {
    pub fn new(page_size: usize) -> Self {
        Self {
            page_handles: HashMap::new(),
            page_errors: HashMap::new(),
            cache: PreviewCache::new(max_cached_image_previews(page_size)),
            request_id: 0,
        }
    }

    pub fn begin_page<I>(&mut self, entries: I) -> (u64, Vec<(usize, String)>)
    where
        I: IntoIterator<Item = (usize, String)>,
    {
        self.request_id = self.request_id.wrapping_add(1);
        self.page_handles.clear();
        self.page_errors.clear();

        let mut missing = Vec::new();
        for (index, line) in entries {
            if let Some(handle) = self.cache.get(&line) {
                self.page_handles.insert(index, handle);
            } else {
                missing.push((index, line));
            }
        }

        (self.request_id, missing)
    }

    pub fn apply_loaded(
        &mut self,
        request_id: u64,
        images: Vec<(usize, String, Result<ImagePreview, String>)>,
    ) -> bool {
        if request_id != self.request_id {
            return false;
        }

        for (index, line, result) in images {
            match result {
                Ok(preview) => {
                    let handle = widget::image::Handle::from_rgba(
                        preview.width,
                        preview.height,
                        preview.pixels,
                    );
                    self.cache.insert(line, handle.clone());
                    self.page_handles.insert(index, handle);
                }
                Err(err) => {
                    self.page_errors.insert(index, err);
                }
            }
        }

        true
    }

    pub fn clear(&mut self) {
        self.page_handles.clear();
        self.page_errors.clear();
        self.cache.clear();
        self.request_id = 0;
    }

    pub fn preview(&self, index: usize) -> Option<&widget::image::Handle> {
        self.page_handles.get(&index)
    }

    pub fn error(&self, index: usize) -> Option<&str> {
        self.page_errors.get(&index).map(String::as_str)
    }
}

#[derive(Debug)]
struct PreviewCache {
    handles: HashMap<String, widget::image::Handle>,
    order: VecDeque<String>,
    capacity: usize,
}

impl PreviewCache {
    fn new(capacity: usize) -> Self {
        Self {
            handles: HashMap::new(),
            order: VecDeque::new(),
            capacity,
        }
    }

    fn get(&mut self, line: &str) -> Option<widget::image::Handle> {
        let handle = self.handles.get(line).cloned();

        if handle.is_some() {
            self.touch(line);
        }

        handle
    }

    fn insert(&mut self, line: String, handle: widget::image::Handle) {
        self.handles.insert(line.clone(), handle);
        self.touch(&line);
        self.evict();
    }

    fn clear(&mut self) {
        self.handles.clear();
        self.order.clear();
    }

    fn touch(&mut self, line: &str) {
        if let Some(position) = self.order.iter().position(|entry| entry == line) {
            self.order.remove(position);
        }
        self.order.push_back(line.to_string());
    }

    fn evict(&mut self) {
        while self.order.len() > self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.handles.remove(&oldest);
            }
        }
    }
}

fn max_cached_image_previews(page_size: usize) -> usize {
    page_size.saturating_mul(6).clamp(32, 192)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle(value: u8) -> widget::image::Handle {
        widget::image::Handle::from_rgba(1, 1, vec![value, value, value, 255])
    }

    #[test]
    fn begin_page_reuses_cached_previews() {
        let mut state = ImageState::new(1);
        let (first_request, first_missing) = state.begin_page(vec![(2, "entry-a".to_string())]);
        assert_eq!(first_missing, vec![(2, "entry-a".to_string())]);
        assert!(state.apply_loaded(
            first_request,
            vec![(
                2,
                "entry-a".to_string(),
                Ok(ImagePreview {
                    width: 1,
                    height: 1,
                    pixels: vec![1, 2, 3, 4],
                })
            )],
        ));

        let (request_id, missing) =
            state.begin_page(vec![(2, "entry-a".to_string()), (4, "entry-b".to_string())]);

        assert_eq!(request_id, 2);
        assert!(state.preview(2).is_some());
        assert_eq!(missing, vec![(4, "entry-b".to_string())]);
    }

    #[test]
    fn cache_evicts_oldest_entries_once_capacity_is_reached() {
        let mut cache = PreviewCache::new(2);
        cache.insert("one".to_string(), handle(1));
        cache.insert("two".to_string(), handle(2));
        cache.insert("three".to_string(), handle(3));

        assert!(cache.get("one").is_none());
        assert!(cache.get("two").is_some());
        assert!(cache.get("three").is_some());
    }
}
