use std::collections::HashMap;

use cosmic::app::{Core, Task};
use cosmic::iced::platform_specific::runtime::wayland::layer_surface::{
    IcedMargin, SctkLayerSurfaceSettings,
};
use cosmic::iced::platform_specific::shell::commands::layer_surface::{
    Anchor, KeyboardInteractivity, Layer, get_layer_surface,
};
use cosmic::iced::widget::scrollable::RelativeOffset;
use cosmic::iced::{Limits, Subscription};
use cosmic::widget;
use cosmic::{Application, Element, executor};
use std::time::Duration;
use std::time::Instant;

use crate::cliphist::{copy_entry, decode_page_images, delete_entry, load_history};
use crate::config::{PAGE_SIZE, WINDOW_HEIGHT, WINDOW_TOP_MARGIN, WINDOW_WIDTH};
use crate::messages::{Message, VimMode};
use crate::models::ClipItem;
use crate::utils::{current_page_indices, next_selected_index, page_count};

pub struct ClipboardApp {
    pub(crate) core: Core,
    pub(crate) items: Vec<ClipItem>,
    pub(crate) filtered: Vec<usize>,
    pub(crate) search_query: String,
    pub(crate) selected: Option<usize>,
    pub(crate) page: usize,
    pub(crate) search_id: widget::Id,
    pub(crate) dummy_id: widget::Id,
    pub(crate) list_id: widget::Id,
    pub(crate) page_images: HashMap<usize, widget::image::Handle>,
    pub(crate) page_image_errors: HashMap<usize, String>,
    pub(crate) page_image_request: u64,
    pub(crate) status: Option<String>,
    pub(crate) vim_mode: Option<VimMode>,
}

impl Application for ClipboardApp {
    type Executor = executor::Default;
    type Flags = bool;
    type Message = Message;
    const APP_ID: &'static str = "com.github.cliprs";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, is_vim: Self::Flags) -> (Self, Task<Self::Message>) {
        core.window.use_template = false;
        core.window.show_headerbar = false;
        core.window.content_container = false;
        core.window.show_maximize = false;
        core.window.show_minimize = false;
        core.window.show_close = false;

        let surface_id = cosmic::iced::window::Id::unique();
        let search_id = widget::Id::unique();
        let dummy_id = widget::Id::unique();
        let list_id = widget::Id::unique();
        let (items, status) = match load_history() {
            Ok(items) => (items, None),
            Err(err) => (Vec::new(), Some(err)),
        };

        let vim_mode = if is_vim { Some(VimMode::Normal) } else { None };

        let mut app = Self {
            core,
            items,
            filtered: Vec::new(),
            search_query: String::new(),
            selected: None,
            page: 0,
            search_id,
            dummy_id,
            list_id,
            page_images: HashMap::new(),
            page_image_errors: HashMap::new(),
            page_image_request: 0,
            status,
            vim_mode,
        };

        app.rebuild_filtered(None);

        let focus_task = if matches!(app.vim_mode, Some(VimMode::Normal)) {
            widget::button::focus(app.dummy_id.clone())
        } else {
            widget::text_input::focus(app.search_id.clone())
        };

        let scroll = app.scroll_to_selection();
        let image_task = app.load_visible_images();

        (
            app,
            Task::batch([
                layer_surface_task(surface_id),
                focus_task,
                scroll,
                image_task,
            ]),
        )
    }

    fn on_escape(&mut self) -> Task<Self::Message> {
        self.close_window()
    }

    fn on_search(&mut self) -> Task<Self::Message> {
        widget::text_input::focus(self.search_id.clone())
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        crate::keyboard::subscription()
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::NoOp => Task::none(),
            Message::EnterNormalMode => {
                if let Some(VimMode::Insert { .. }) = self.vim_mode {
                    self.vim_mode = Some(VimMode::Normal);
                    Task::batch([widget::button::focus(self.dummy_id.clone())])
                } else {
                    Task::none()
                }
            }
            Message::EnterInsertMode => {
                if let Some(VimMode::Normal) = self.vim_mode {
                    self.vim_mode = Some(VimMode::Insert { last_j: None });
                    Task::batch([
                        widget::text_input::focus(self.search_id.clone()),
                        widget::text_input::move_cursor_to_end(self.search_id.clone()),
                    ])
                } else {
                    Task::none()
                }
            }
            Message::GlobalEscape => {
                if let Some(VimMode::Insert { .. }) = self.vim_mode {
                    self.update(Message::EnterNormalMode)
                } else {
                    self.update(Message::CloseWindow)
                }
            }
            Message::HandleVimAction(action) => self.handle_vim_action(action),
            Message::SearchChanged(query) => {
                if let Some(VimMode::Normal) = self.vim_mode {
                    return Task::none();
                }

                if let Some(VimMode::Insert { last_j }) = &mut self.vim_mode {
                    if query.ends_with("jk")
                        && let Some(j_time) = last_j
                        && Instant::now().duration_since(*j_time) < Duration::from_millis(300)
                    {
                        self.search_query = query[..query.len() - 2].to_string();
                        self.vim_mode = Some(VimMode::Normal);
                        self.status = None;
                        self.rebuild_filtered(None);
                        return Task::batch([
                            widget::button::focus(self.dummy_id.clone()),
                            self.scroll_to_selection(),
                            self.load_visible_images(),
                        ]);
                    }
                    if query.ends_with('j') {
                        *last_j = Some(Instant::now());
                    } else {
                        *last_j = None;
                    }
                }

                self.search_query = query;
                self.status = None;
                self.rebuild_filtered(None);
                let image_task = self.load_visible_images();

                Task::batch([
                    widget::text_input::focus(self.search_id.clone()),
                    self.scroll_to_selection(),
                    image_task,
                ])
            }
            Message::ClearSearch => {
                self.search_query.clear();
                self.status = None;
                self.rebuild_filtered(None);
                let image_task = self.load_visible_images();

                Task::batch([
                    widget::text_input::focus(self.search_id.clone()),
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
            Message::ActivateSelection => self.copy_selected(),
            Message::SelectAndActivate(index) => {
                self.selected = Some(index);
                self.copy_selected()
            }
            Message::Reload => self.reload_history(),
            Message::DeleteSelected => self.delete_selected(),
            Message::CloseWindow => self.close_window(),
            Message::CopyDone(Ok(())) => self.close_window(),
            Message::CopyDone(Err(err)) => {
                self.status = Some(err);
                Task::none()
            }
            Message::PageImagesLoaded { request_id, images } => {
                if request_id != self.page_image_request {
                    return Task::none();
                }

                self.page_images.clear();
                self.page_image_errors.clear();

                for (index, result) in images {
                    match result {
                        Ok(bytes) => {
                            self.page_images
                                .insert(index, widget::image::Handle::from_bytes(bytes));
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
        let total_pages = page_count(self.filtered.len());

        if total_pages == 0 {
            self.page = 0;
            return;
        }

        self.page = self.page.min(total_pages - 1);

        if let Some(selected) = self.selected {
            if let Some(position) = self.filtered.iter().position(|index| *index == selected) {
                self.page = position / PAGE_SIZE;
            }
        }
    }

    pub(crate) fn change_page(&mut self, delta: isize) -> bool {
        let total_pages = page_count(self.filtered.len());
        if total_pages == 0 {
            self.page = 0;
            self.selected = None;
            return false;
        }

        let next_page = (self.page as isize + delta).clamp(0, total_pages as isize - 1) as usize;
        let changed = next_page != self.page;
        self.page = next_page;

        let visible = current_page_indices(&self.filtered, self.page);
        if !self.selected.is_some_and(|index| visible.contains(&index)) {
            self.selected = visible.first().copied();
        }

        changed
    }

    pub(crate) fn scroll_to_selection(&self) -> Task<Message> {
        let visible = current_page_indices(&self.filtered, self.page);
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
        let visible_images: Vec<(usize, String)> = current_page_indices(&self.filtered, self.page)
            .iter()
            .copied()
            .filter(|index| self.items[*index].kind.is_image())
            .map(|index| (index, self.items[index].line.clone()))
            .collect();

        if visible_images.is_empty() {
            return Task::none();
        }

        Task::perform(decode_page_images(visible_images), move |images| {
            cosmic::Action::App(Message::PageImagesLoaded { request_id, images })
        })
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
        Task::perform(async move { copy_entry(&item) }, |res| {
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

        self.rebuild_filtered(preferred_line.as_deref());
        let image_task = self.load_visible_images();

        Task::batch([self.scroll_to_selection(), image_task])
    }

    pub(crate) fn delete_selected(&mut self) -> Task<Message> {
        let Some(item) = self.selected_item().cloned() else {
            self.status = Some("Nothing is selected.".to_string());
            return Task::none();
        };

        match delete_entry(&item.line) {
            Ok(()) => self.reload_history(),
            Err(err) => {
                self.status = Some(err);
                Task::none()
            }
        }
    }
}

pub fn layer_surface_task(surface_id: cosmic::iced::window::Id) -> Task<Message> {
    let mut surface = SctkLayerSurfaceSettings::default();
    surface.id = surface_id;
    surface.layer = Layer::Overlay;
    surface.keyboard_interactivity = KeyboardInteractivity::OnDemand;
    surface.anchor = Anchor::TOP;
    surface.margin = IcedMargin {
        top: WINDOW_TOP_MARGIN,
        ..IcedMargin::default()
    };
    surface.namespace = "cliprs".to_string();
    surface.size = Some((Some(WINDOW_WIDTH as u32), Some(WINDOW_HEIGHT as u32)));
    surface.exclusive_zone = 0;
    surface.size_limits = Limits::NONE
        .min_width(WINDOW_WIDTH)
        .max_width(WINDOW_WIDTH)
        .min_height(WINDOW_HEIGHT)
        .max_height(WINDOW_HEIGHT);

    get_layer_surface(surface)
}
