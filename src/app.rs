use cosmic::app::{Core, Task};
use cosmic::iced::Subscription;
use cosmic::iced::widget::scrollable::RelativeOffset;
use cosmic::widget;
use cosmic::{Application, ApplicationExt, Element, executor};
use std::time::Duration;
use std::time::Instant;

use crate::cliphist::{copy_entry, decode_page_images, delete_entry, load_history, wipe_history};
use crate::config::Config;
use crate::image_state::ImageState;
use crate::messages::{Message, SelectionMove, VimMode};
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
    pub(crate) image_state: ImageState,
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
            image_state: ImageState::new(config.page_size),
            status,
            vim_mode,
            menu_open: false,
        };

        app.rebuild_filtered(None);

        let focus_task = app.focus_task(focus_target_for_mode(app.vim_mode.as_ref()));

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
                    self.focus_task(FocusTarget::ModeSink)
                } else {
                    Task::none()
                }
            }
            Message::EnterInsertMode => {
                if let Some(VimMode::Normal) = self.vim_mode {
                    self.vim_mode = Some(VimMode::Insert { last_j: None });
                    self.focus_task(FocusTarget::SearchInput)
                } else {
                    Task::none()
                }
            }
            Message::GlobalEscape => self.update(escape_outcome(self.vim_mode.as_ref())),
            Message::HandleVimAction(action) => self.handle_vim_action(action),
            Message::SearchChanged(query) => self.update_search(query),
            Message::ClearSearch => {
                self.apply_search_query(String::new(), FocusTarget::SearchInput)
            }
            Message::MoveSelection(movement) => {
                let previous_page = self.page;
                self.move_selection(movement);
                self.sync_page_to_selection();
                self.status = None;
                self.refresh_visible_content(self.page != previous_page)
            }
            Message::PrevPage => {
                let changed = self.change_page(-1);
                self.status = None;
                self.refresh_visible_content(changed)
            }
            Message::NextPage => {
                let changed = self.change_page(1);
                self.status = None;
                self.refresh_visible_content(changed)
            }
            Message::ActivateSelection => {
                self.close_menu();
                self.copy_selected()
            }
            Message::SelectAndActivate(index) => {
                self.close_menu();
                self.selected = Some(index);
                self.copy_selected()
            }
            Message::Reload => {
                self.close_menu();
                self.reload_history()
            }
            Message::DeleteSelected => {
                self.close_menu();
                self.delete_selected()
            }
            Message::CloseWindow => self.close_window(),
            Message::ToggleMenu => {
                self.menu_open = !self.menu_open;
                Task::none()
            }
            Message::WipeHistory => {
                self.close_menu();
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
                self.image_state.apply_loaded(request_id, images);
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

    pub(crate) fn total_pages(&self) -> usize {
        page_count(self.filtered.len(), self.config.page_size)
    }

    pub(crate) fn visible_indices(&self) -> &[usize] {
        current_page_indices(&self.filtered, self.page, self.config.page_size)
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

    pub(crate) fn move_selection(&mut self, movement: SelectionMove) {
        self.selected = next_selected_index(&self.filtered, self.selected, movement);
    }

    pub(crate) fn sync_page_to_selection(&mut self) {
        let total_pages = self.total_pages();

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

        let visible = self.visible_indices();
        if !self.selected.is_some_and(|index| visible.contains(&index)) {
            self.selected = visible.first().copied();
        }

        changed
    }

    pub(crate) fn scroll_to_selection(&self) -> Task<Message> {
        let visible = self.visible_indices();
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
        let visible_images = self
            .visible_indices()
            .iter()
            .copied()
            .filter(|index| self.items[*index].kind.is_image())
            .map(|index| (index, self.items[index].line.clone()))
            .collect::<Vec<_>>();
        let (request_id, visible_images) = self.image_state.begin_page(visible_images);

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

        self.image_state.clear();
        self.rebuild_filtered(preferred_line.as_deref());
        self.refresh_visible_content(true)
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

    fn update_search(&mut self, query: String) -> Task<Message> {
        match search_change_action(self.vim_mode.as_ref(), &query, Instant::now()) {
            SearchChangeAction::Ignore => Task::none(),
            SearchChangeAction::ExitInsert { trimmed_query } => {
                self.vim_mode = Some(VimMode::Normal);
                self.apply_search_query(trimmed_query, FocusTarget::ModeSink)
            }
            SearchChangeAction::Apply { query, next_last_j } => {
                if let Some(VimMode::Insert { last_j }) = &mut self.vim_mode {
                    *last_j = next_last_j;
                }

                self.apply_search_query(query, FocusTarget::SearchInput)
            }
        }
    }

    fn apply_search_query(&mut self, query: String, focus_target: FocusTarget) -> Task<Message> {
        self.search_query = query;
        self.status = None;
        self.rebuild_filtered(None);
        self.refresh_after_filter_change(focus_target)
    }

    fn refresh_after_filter_change(&mut self, focus_target: FocusTarget) -> Task<Message> {
        let focus = self.focus_task(focus_target);
        let scroll = self.scroll_to_selection();
        let images = self.load_visible_images();

        Task::batch([focus, scroll, images])
    }

    fn refresh_visible_content(&mut self, reload_images: bool) -> Task<Message> {
        let scroll = self.scroll_to_selection();
        let images = if reload_images {
            self.load_visible_images()
        } else {
            Task::none()
        };

        Task::batch([scroll, images])
    }

    fn focus_task(&self, target: FocusTarget) -> Task<Message> {
        focus_task_for_mode(target, self.search_id.clone(), self.dummy_id.clone())
    }

    fn close_menu(&mut self) {
        self.menu_open = false;
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
