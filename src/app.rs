use std::collections::{HashMap, VecDeque};

use cosmic::app::{Core, Task};
use cosmic::iced::Subscription;
use cosmic::iced::widget::scrollable::RelativeOffset;
use cosmic::widget;
use cosmic::{Application, ApplicationExt, Element, executor};
use std::time::Duration;
use std::time::Instant;

use crate::cliphist::{copy_entry, decode_page_images, delete_entry, load_history, wipe_history};
use crate::config::Config;
use crate::messages::{Message, VimMode};
use crate::models::ClipItem;
use crate::utils::{current_page_indices, next_selected_index, page_count};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusTarget {
    SearchInput,
    ModeSink,
}

#[derive(Clone, Debug, PartialEq)]
enum SearchChangeAction {
    Ignore,
    Apply {
        query: String,
        next_last_j: Option<Instant>,
    },
    ExitInsert {
        trimmed_query: String,
    },
}

pub struct ClipboardApp {
    pub(crate) core: Core,
    pub(crate) config: Config,
    pub(crate) items: Vec<ClipItem>,
    pub(crate) filtered: Vec<usize>,
    pub(crate) search_query: String,
    pub(crate) selected: Option<usize>,
    pub(crate) page: usize,
    pub(crate) search_id: widget::Id,
    pub(crate) dummy_id: widget::Id,
    pub(crate) list_id: widget::Id,
    pub(crate) page_images: HashMap<usize, widget::image::Handle>,
    pub(crate) image_preview_cache: HashMap<String, widget::image::Handle>,
    pub(crate) image_preview_order: VecDeque<String>,
    pub(crate) page_image_errors: HashMap<usize, String>,
    pub(crate) page_image_request: u64,
    pub(crate) status: Option<String>,
    pub(crate) vim_mode: Option<VimMode>,
    pub(crate) menu_open: bool,
}

impl Application for ClipboardApp {
    type Executor = executor::Default;
    type Flags = (bool, Config);
    type Message = Message;
    const APP_ID: &'static str = "com.github.cliphist_cosmic";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, (is_vim, config): Self::Flags) -> (Self, Task<Self::Message>) {
        core.window.use_template = false;
        core.window.show_headerbar = false;
        core.window.content_container = false;
        core.window.show_maximize = false;
        core.window.show_minimize = false;
        core.window.show_close = false;

        let search_id = widget::Id::unique();
        let dummy_id = widget::Id::unique();
        let list_id = widget::Id::unique();
        let (items, status) = match load_history() {
            Ok(items) => (items, None),
            Err(err) => (Vec::new(), Some(err)),
        };

        let vim_mode = initial_vim_mode(is_vim);

        let mut app = Self {
            core,
            config,
            items,
            filtered: Vec::new(),
            search_query: String::new(),
            selected: None,
            page: 0,
            search_id,
            dummy_id,
            list_id,
            page_images: HashMap::new(),
            image_preview_cache: HashMap::new(),
            image_preview_order: VecDeque::new(),
            page_image_errors: HashMap::new(),
            page_image_request: 0,
            status,
            vim_mode,
            menu_open: false,
        };

        app.rebuild_filtered(None);

        let focus_task = focus_task_for_mode(
            focus_target_for_mode(app.vim_mode.as_ref()),
            app.search_id.clone(),
            app.dummy_id.clone(),
        );

        let scroll = app.scroll_to_selection();
        let image_task = app.load_visible_images();
        let init_task = if app.config.uses_layer_surface() {
            app.config
                .layer_surface_task::<Message>()
                .chain(Task::batch([focus_task, scroll, image_task]))
        } else {
            Task::batch([focus_task, scroll, image_task])
        };

        (app, init_task)
    }

    fn on_escape(&mut self) -> Task<Self::Message> {
        self.close_window()
    }

    fn on_search(&mut self) -> Task<Self::Message> {
        widget::text_input::focus(self.search_id.clone())
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        crate::keyboard::subscription(self.vim_mode.is_some())
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::NoOp => Task::none(),
            Message::DragWindow => self.drag(),
            Message::InsertSearchText(text) => {
                if matches!(self.vim_mode, Some(VimMode::Normal)) || text.is_empty() {
                    Task::none()
                } else {
                    let mut query = self.search_query.clone();
                    query.push_str(&text);
                    self.update(Message::SearchChanged(query))
                }
            }
            Message::EnterNormalMode => {
                if let Some(VimMode::Insert { .. }) = self.vim_mode {
                    self.vim_mode = Some(VimMode::Normal);
                    focus_task_for_mode(
                        FocusTarget::ModeSink,
                        self.search_id.clone(),
                        self.dummy_id.clone(),
                    )
                } else {
                    Task::none()
                }
            }
            Message::EnterInsertMode => {
                if let Some(VimMode::Normal) = self.vim_mode {
                    self.vim_mode = Some(VimMode::Insert { last_j: None });
                    focus_task_for_mode(
                        FocusTarget::SearchInput,
                        self.search_id.clone(),
                        self.dummy_id.clone(),
                    )
                } else {
                    Task::none()
                }
            }
            Message::GlobalEscape => self.update(escape_outcome(self.vim_mode.as_ref())),
            Message::HandleVimAction(action) => self.handle_vim_action(action),
            Message::SearchChanged(query) => {
                match search_change_action(self.vim_mode.as_ref(), &query, Instant::now()) {
                    SearchChangeAction::Ignore => Task::none(),
                    SearchChangeAction::ExitInsert { trimmed_query } => {
                        self.search_query = trimmed_query;
                        self.vim_mode = Some(VimMode::Normal);
                        self.status = None;
                        self.rebuild_filtered(None);
                        Task::batch([
                            focus_task_for_mode(
                                FocusTarget::ModeSink,
                                self.search_id.clone(),
                                self.dummy_id.clone(),
                            ),
                            self.scroll_to_selection(),
                            self.load_visible_images(),
                        ])
                    }
                    SearchChangeAction::Apply { query, next_last_j } => {
                        if let Some(VimMode::Insert { last_j }) = &mut self.vim_mode {
                            *last_j = next_last_j;
                        }

                        self.search_query = query;
                        self.status = None;
                        self.rebuild_filtered(None);
                        let image_task = self.load_visible_images();

                        Task::batch([
                            focus_task_for_mode(
                                FocusTarget::SearchInput,
                                self.search_id.clone(),
                                self.dummy_id.clone(),
                            ),
                            self.scroll_to_selection(),
                            image_task,
                        ])
                    }
                }
            }
            Message::ClearSearch => {
                self.search_query.clear();
                self.status = None;
                self.rebuild_filtered(None);
                let image_task = self.load_visible_images();

                Task::batch([
                    focus_task_for_mode(
                        FocusTarget::SearchInput,
                        self.search_id.clone(),
                        self.dummy_id.clone(),
                    ),
                    self.scroll_to_selection(),
                    image_task,
                ])
            }
            Message::MoveSelection(delta) => {
                let previous_page = self.page;
                self.move_selection(delta);
                self.sync_page_to_selection();
                self.status = None;
                let image_task = if self.page != previous_page {
                    self.load_visible_images()
                } else {
                    Task::none()
                };

                Task::batch([self.scroll_to_selection(), image_task])
            }
            Message::PrevPage => {
                let changed = self.change_page(-1);
                self.status = None;
                let image_task = if changed {
                    self.load_visible_images()
                } else {
                    Task::none()
                };

                Task::batch([self.scroll_to_selection(), image_task])
            }
            Message::NextPage => {
                let changed = self.change_page(1);
                self.status = None;
                let image_task = if changed {
                    self.load_visible_images()
                } else {
                    Task::none()
                };

                Task::batch([self.scroll_to_selection(), image_task])
            }
            Message::ActivateSelection => {
                self.menu_open = false;
                self.copy_selected()
            }
            Message::SelectAndActivate(index) => {
                self.menu_open = false;
                self.selected = Some(index);
                self.copy_selected()
            }
            Message::Reload => {
                self.menu_open = false;
                self.reload_history()
            }
            Message::DeleteSelected => {
                self.menu_open = false;
                self.delete_selected()
            }
            Message::CloseWindow => self.close_window(),
            Message::ToggleMenu => {
                self.menu_open = !self.menu_open;
                Task::none()
            }
            Message::WipeHistory => {
                self.menu_open = false;
                self.status = Some("Wiping history...".to_string());
                Task::perform(wipe_history(), |res| {
                    cosmic::Action::App(Message::WipeDone(res))
                })
            }
            Message::WipeDone(Ok(())) => self.reload_history(),
            Message::WipeDone(Err(err)) => {
                self.status = Some(err);
                Task::none()
            }
            Message::CopyDone(Ok(())) => self.close_window(),
            Message::CopyDone(Err(err)) => {
                self.status = Some(err);
                Task::none()
            }
            Message::DeleteDone(Ok(())) => self.reload_history(),
            Message::DeleteDone(Err(err)) => {
                self.status = Some(err);
                Task::none()
            }
            Message::PageImagesLoaded { request_id, images } => {
                if request_id != self.page_image_request {
                    return Task::none();
                }

                for (index, line, result) in images {
                    match result {
                        Ok(preview) => {
                            let handle = widget::image::Handle::from_rgba(
                                preview.width,
                                preview.height,
                                preview.pixels,
                            );
                            self.cache_image_preview(line, handle.clone());
                            self.page_images.insert(index, handle);
                        }
                        Err(err) => {
                            self.page_image_errors.insert(index, err);
                        }
                    }
                }

                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        self.content_view()
    }

    fn view_window(&self, _id: cosmic::iced::window::Id) -> Element<'_, Self::Message> {
        self.content_view()
    }
}

impl ClipboardApp {
    pub(crate) fn close_window(&self) -> Task<Message> {
        cosmic::iced::exit()
    }

    pub(crate) fn rebuild_filtered(&mut self, preferred_line: Option<&str>) {
        let query = self.search_query.trim().to_lowercase();

        self.filtered = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| query.is_empty() || item.preview_lower.contains(&query))
            .map(|(index, _)| index)
            .collect();

        self.coerce_selection(preferred_line);
        self.sync_page_to_selection();
    }

    pub(crate) fn selected_item(&self) -> Option<&ClipItem> {
        self.selected.and_then(|index| self.items.get(index))
    }

    pub(crate) fn coerce_selection(&mut self, preferred_line: Option<&str>) {
        self.selected = preferred_line
            .and_then(|line| {
                self.filtered
                    .iter()
                    .copied()
                    .find(|index| self.items[*index].line == line)
            })
            .or_else(|| self.selected.filter(|index| self.filtered.contains(index)))
            .or_else(|| self.filtered.first().copied());
    }

    pub(crate) fn move_selection(&mut self, delta: i32) {
        self.selected = next_selected_index(&self.filtered, self.selected, delta);
    }

    pub(crate) fn sync_page_to_selection(&mut self) {
        let total_pages = page_count(self.filtered.len(), self.config.page_size);

        if total_pages == 0 {
            self.page = 0;
            return;
        }

        self.page = self.page.min(total_pages - 1);

        if let Some(selected) = self.selected {
            if let Some(position) = self.filtered.iter().position(|index| *index == selected) {
                self.page = position / self.config.page_size;
            }
        }
    }

    pub(crate) fn change_page(&mut self, delta: isize) -> bool {
        let total_pages = page_count(self.filtered.len(), self.config.page_size);
        if total_pages == 0 {
            self.page = 0;
            self.selected = None;
            return false;
        }

        let next_page = (self.page as isize + delta).clamp(0, total_pages as isize - 1) as usize;
        let changed = next_page != self.page;
        self.page = next_page;

        let visible = current_page_indices(&self.filtered, self.page, self.config.page_size);
        if !self.selected.is_some_and(|index| visible.contains(&index)) {
            self.selected = visible.first().copied();
        }

        changed
    }

    pub(crate) fn scroll_to_selection(&self) -> Task<Message> {
        let visible = current_page_indices(&self.filtered, self.page, self.config.page_size);
        let Some(selected) = self.selected else {
            return Task::none();
        };
        let Some(position) = visible.iter().position(|index| *index == selected) else {
            return Task::none();
        };

        let ratio = if visible.len() <= 1 {
            0.0
        } else {
            position as f32 / (visible.len() - 1) as f32
        };

        cosmic::iced::widget::scrollable::snap_to(
            self.list_id.clone(),
            RelativeOffset {
                x: Some(0.0),
                y: Some(ratio),
            },
        )
    }

    pub(crate) fn load_visible_images(&mut self) -> Task<Message> {
        self.page_image_request = self.page_image_request.wrapping_add(1);
        self.page_images.clear();
        self.page_image_errors.clear();

        let request_id = self.page_image_request;
        let mut visible_images = Vec::new();
        let visible_indices: Vec<usize> =
            current_page_indices(&self.filtered, self.page, self.config.page_size)
                .iter()
                .copied()
                .filter(|index| self.items[*index].kind.is_image())
                .collect();

        for index in visible_indices {
            let line = self.items[index].line.clone();

            if let Some(handle) = self.cached_image_preview(&line) {
                self.page_images.insert(index, handle);
            } else {
                visible_images.push((index, line));
            }
        }

        if visible_images.is_empty() {
            return Task::none();
        }

        let preview_width = self.config.window_width;
        let preview_height = self.config.image_height;

        Task::perform(
            decode_page_images(visible_images, preview_width, preview_height),
            move |images| cosmic::Action::App(Message::PageImagesLoaded { request_id, images }),
        )
    }

    pub(crate) fn copy_selected(&mut self) -> Task<Message> {
        if self.status.as_deref() == Some("Copying...") {
            return Task::none();
        }

        let Some(item) = self.selected_item().cloned() else {
            self.status = Some("Nothing is selected.".to_string());
            return Task::none();
        };

        self.status = Some("Copying...".to_string());
        Task::perform(async move { copy_entry(&item).await }, |res| {
            cosmic::Action::App(Message::CopyDone(res))
        })
    }

    pub(crate) fn reload_history(&mut self) -> Task<Message> {
        let preferred_line = self.selected_item().map(|item| item.line.clone());

        match load_history() {
            Ok(items) => {
                self.items = items;
                self.status = None;
            }
            Err(err) => {
                self.items.clear();
                self.status = Some(err);
            }
        }

        self.image_preview_cache.clear();
        self.image_preview_order.clear();
        self.rebuild_filtered(preferred_line.as_deref());
        let image_task = self.load_visible_images();

        Task::batch([self.scroll_to_selection(), image_task])
    }

    pub(crate) fn delete_selected(&mut self) -> Task<Message> {
        let Some(item) = self.selected_item().cloned() else {
            self.status = Some("Nothing is selected.".to_string());
            return Task::none();
        };

        self.status = Some("Deleting...".to_string());
        Task::perform(async move { delete_entry(&item.line).await }, |res| {
            cosmic::Action::App(Message::DeleteDone(res))
        })
    }

    fn cached_image_preview(&mut self, line: &str) -> Option<widget::image::Handle> {
        let handle = self.image_preview_cache.get(line).cloned();

        if handle.is_some() {
            self.touch_cached_image_preview(line);
        }

        handle
    }

    fn cache_image_preview(&mut self, line: String, handle: widget::image::Handle) {
        self.image_preview_cache.insert(line.clone(), handle);
        self.touch_cached_image_preview(&line);

        while self.image_preview_order.len() > self.max_cached_image_previews() {
            if let Some(oldest) = self.image_preview_order.pop_front() {
                self.image_preview_cache.remove(&oldest);
            }
        }
    }

    fn touch_cached_image_preview(&mut self, line: &str) {
        self.image_preview_order.retain(|entry| entry != line);
        self.image_preview_order.push_back(line.to_string());
    }

    fn max_cached_image_previews(&self) -> usize {
        self.config.page_size.saturating_mul(6).clamp(32, 192)
    }
}

fn initial_vim_mode(is_vim: bool) -> Option<VimMode> {
    if is_vim { Some(VimMode::Normal) } else { None }
}

fn focus_target_for_mode(vim_mode: Option<&VimMode>) -> FocusTarget {
    if matches!(vim_mode, Some(VimMode::Normal)) {
        FocusTarget::ModeSink
    } else {
        FocusTarget::SearchInput
    }
}

fn focus_task_for_mode(
    target: FocusTarget,
    search_id: widget::Id,
    dummy_id: widget::Id,
) -> Task<Message> {
    match target {
        FocusTarget::SearchInput => Task::batch([
            widget::text_input::focus(search_id.clone()),
            widget::text_input::move_cursor_to_end(search_id),
        ]),
        FocusTarget::ModeSink => widget::button::focus(dummy_id),
    }
}

fn escape_outcome(vim_mode: Option<&VimMode>) -> Message {
    if matches!(vim_mode, Some(VimMode::Insert { .. })) {
        Message::EnterNormalMode
    } else {
        Message::CloseWindow
    }
}

fn search_change_action(
    vim_mode: Option<&VimMode>,
    query: &str,
    now: Instant,
) -> SearchChangeAction {
    match vim_mode {
        Some(VimMode::Normal) => SearchChangeAction::Ignore,
        Some(VimMode::Insert { last_j })
            if query.ends_with("jk")
                && last_j.is_some_and(|j_time| {
                    now.duration_since(j_time) < Duration::from_millis(300)
                }) =>
        {
            SearchChangeAction::ExitInsert {
                trimmed_query: query[..query.len() - 2].to_string(),
            }
        }
        Some(VimMode::Insert { .. }) => SearchChangeAction::Apply {
            query: query.to_string(),
            next_last_j: query.ends_with('j').then_some(now),
        },
        None => SearchChangeAction::Apply {
            query: query.to_string(),
            next_last_j: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vim_mode_starts_in_normal_when_enabled() {
        assert_eq!(initial_vim_mode(true), Some(VimMode::Normal));
        assert_eq!(initial_vim_mode(false), None);
    }

    #[test]
    fn focus_target_tracks_mode() {
        assert_eq!(
            focus_target_for_mode(Some(&VimMode::Normal)),
            FocusTarget::ModeSink
        );
        assert_eq!(
            focus_target_for_mode(Some(&VimMode::Insert { last_j: None })),
            FocusTarget::SearchInput
        );
        assert_eq!(focus_target_for_mode(None), FocusTarget::SearchInput);
    }

    #[test]
    fn escape_leaves_insert_before_closing() {
        assert_eq!(
            escape_outcome(Some(&VimMode::Insert { last_j: None })),
            Message::EnterNormalMode
        );
        assert_eq!(escape_outcome(Some(&VimMode::Normal)), Message::CloseWindow);
        assert_eq!(escape_outcome(None), Message::CloseWindow);
    }

    #[test]
    fn normal_mode_ignores_search_changes() {
        let now = Instant::now();
        assert_eq!(
            search_change_action(Some(&VimMode::Normal), "hello", now),
            SearchChangeAction::Ignore
        );
    }

    #[test]
    fn insert_mode_tracks_j_for_jk_escape() {
        let now = Instant::now();
        assert_eq!(
            search_change_action(Some(&VimMode::Insert { last_j: None }), "j", now),
            SearchChangeAction::Apply {
                query: "j".to_string(),
                next_last_j: Some(now),
            }
        );
        assert_eq!(
            search_change_action(
                Some(&VimMode::Insert { last_j: Some(now) }),
                "hello",
                now + Duration::from_millis(100),
            ),
            SearchChangeAction::Apply {
                query: "hello".to_string(),
                next_last_j: None,
            }
        );
    }

    #[test]
    fn jk_exits_insert_mode_only_within_threshold() {
        let now = Instant::now();
        assert_eq!(
            search_change_action(
                Some(&VimMode::Insert {
                    last_j: Some(now - Duration::from_millis(100)),
                }),
                "abcjk",
                now,
            ),
            SearchChangeAction::ExitInsert {
                trimmed_query: "abc".to_string(),
            }
        );
        assert_eq!(
            search_change_action(
                Some(&VimMode::Insert {
                    last_j: Some(now - Duration::from_millis(400)),
                }),
                "abcjk",
                now,
            ),
            SearchChangeAction::Apply {
                query: "abcjk".to_string(),
                next_last_j: None,
            }
        );
    }

    #[test]
    fn non_vim_search_changes_apply_without_insert_state() {
        let now = Instant::now();
        assert_eq!(
            search_change_action(None, "term", now),
            SearchChangeAction::Apply {
                query: "term".to_string(),
                next_last_j: None,
            }
        );
    }
}
