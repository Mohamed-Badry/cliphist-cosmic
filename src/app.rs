use std::collections::HashMap;

use cosmic::app::{Core, Task};
use cosmic::iced::event::{self, Event};
use cosmic::iced::keyboard::{self, Key, key::Named};
use cosmic::iced::platform_specific::runtime::wayland::layer_surface::{
    IcedMargin, SctkLayerSurfaceSettings,
};
use cosmic::iced::platform_specific::shell::commands::layer_surface::{
    Anchor, KeyboardInteractivity, Layer, get_layer_surface,
};
use cosmic::iced::widget::scrollable::RelativeOffset;
use cosmic::iced::{ContentFit, Length, Limits, Subscription};
use cosmic::theme::Button as ButtonStyle;
use cosmic::widget;
use cosmic::{Application, Element, executor};

use crate::cliphist::{copy_entry, decode_page_images, delete_entry, load_history};
use crate::config::{IMAGE_HEIGHT, PAGE_SIZE, WINDOW_HEIGHT, WINDOW_TOP_MARGIN, WINDOW_WIDTH};
use crate::models::ClipItem;
use crate::utils::{current_page_indices, next_selected_index, page_count};

#[derive(Clone, Debug)]
pub enum Message {
    SearchChanged(String),
    ClearSearch,
    MoveSelection(i32),
    PrevPage,
    NextPage,
    ActivateSelection,
    SelectIndex(usize),
    Reload,
    DeleteSelected,
    CloseWindow,
    CopyDone(Result<(), String>),
    PageImagesLoaded {
        request_id: u64,
        images: Vec<(usize, Result<Vec<u8>, String>)>,
    },
}

pub struct ClipboardApp {
    core: Core,
    items: Vec<ClipItem>,
    filtered: Vec<usize>,
    search_query: String,
    selected: Option<usize>,
    page: usize,
    search_id: widget::Id,
    list_id: widget::Id,
    page_images: HashMap<usize, widget::image::Handle>,
    page_image_errors: HashMap<usize, String>,
    page_image_request: u64,
    status: Option<String>,
}

impl Application for ClipboardApp {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.crim.Cliprs";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        core.window.use_template = false;
        core.window.show_headerbar = false;
        core.window.content_container = false;
        core.window.show_maximize = false;
        core.window.show_minimize = false;
        core.window.show_close = false;

        let surface_id = cosmic::iced::window::Id::unique();
        let search_id = widget::Id::unique();
        let list_id = widget::Id::unique();
        let (items, status) = match load_history() {
            Ok(items) => (items, None),
            Err(err) => (Vec::new(), Some(err)),
        };

        let mut app = Self {
            core,
            items,
            filtered: Vec::new(),
            search_query: String::new(),
            selected: None,
            page: 0,
            search_id,
            list_id,
            page_images: HashMap::new(),
            page_image_errors: HashMap::new(),
            page_image_request: 0,
            status,
        };

        app.rebuild_filtered(None);

        let search_focus = widget::text_input::focus(app.search_id.clone());
        let scroll = app.scroll_to_selection();
        let image_task = app.load_visible_images();

        (
            app,
            Task::batch([
                layer_surface_task(surface_id),
                search_focus,
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
        event::listen_with(|event, status, _id| match event {
            Event::Window(cosmic::iced::window::Event::Unfocused) => Some(Message::CloseWindow),
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                if key == Key::Named(Named::Escape) {
                    return Some(Message::CloseWindow);
                }

                if key == Key::Named(Named::Enter) {
                    return Some(Message::ActivateSelection);
                }

                if status == event::Status::Ignored {
                    match key {
                        Key::Named(Named::ArrowDown) => Some(Message::MoveSelection(1)),
                        Key::Named(Named::ArrowUp) => Some(Message::MoveSelection(-1)),
                        Key::Named(Named::PageDown) => Some(Message::NextPage),
                        Key::Named(Named::PageUp) => Some(Message::PrevPage),
                        Key::Named(Named::Home) => Some(Message::MoveSelection(i32::MIN)),
                        Key::Named(Named::End) => Some(Message::MoveSelection(i32::MAX)),
                        Key::Named(Named::Delete)
                            if !modifiers.control()
                                && !modifiers.alt()
                                && !modifiers.shift()
                                && !modifiers.logo() =>
                        {
                            Some(Message::DeleteSelected)
                        }
                        Key::Character(ch)
                            if modifiers.control() && ch.as_str().eq_ignore_ascii_case("r") =>
                        {
                            Some(Message::Reload)
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::SearchChanged(query) => {
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
            Message::SelectIndex(index) => {
                self.selected = Some(index);
                self.sync_page_to_selection();
                Task::none()
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
    fn content_view(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let total_pages = page_count(self.filtered.len());
        let visible = current_page_indices(&self.filtered, self.page);

        let search = widget::text_input::search_input("Search clipboard", &self.search_query)
            .on_input(Message::SearchChanged)
            .on_clear(Message::ClearSearch)
            .id(self.search_id.clone())
            .width(Length::Fill);

        let results_label = if self.filtered.is_empty() {
            "No results".to_string()
        } else {
            format!("{} results", self.filtered.len())
        };
        let page_label = if total_pages == 0 {
            "Page 0/0".to_string()
        } else {
            format!("Page {}/{}", self.page + 1, total_pages)
        };

        let mut list = widget::column().spacing(spacing.space_s);
        for index in visible.iter().copied() {
            list = list.push(self.render_item(index));
        }

        if visible.is_empty() {
            list = list.push(
                widget::container(widget::text(
                    "No clipboard entries match the current search.",
                ))
                .padding(spacing.space_s),
            );
        }

        let status = self.status.as_deref().unwrap_or(
            "Enter copies | PageUp/PageDown switches page | Del deletes | Ctrl+R reloads | Esc closes",
        );

        let content = widget::container(
            widget::column()
                .spacing(spacing.space_m)
                .padding([spacing.space_m, spacing.space_l])
                .push(search)
                .push(
                    widget::row()
                        .push(widget::text(results_label).size(13))
                        .push(widget::space::horizontal())
                        .push(widget::text(page_label).size(13)),
                )
                .push(
                    widget::scrollable(list)
                        .id(self.list_id.clone())
                        .height(Length::Fill),
                )
                .push(widget::text(status).size(12)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .class(cosmic::theme::Container::Background);

        widget::layer_container(content).into()
    }

    fn render_item(&self, index: usize) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let item = &self.items[index];
        let selected = self.selected == Some(index);
        let class = if selected {
            ButtonStyle::Suggested
        } else {
            ButtonStyle::Standard
        };
        let header = widget::row()
            .push(widget::text(item.kind.label()).size(11))
            .push(widget::space::horizontal())
            .push(widget::text(format!("#{}", item.id)).size(11));

        if item.kind.is_image() {
            let preview: Element<'_, Message> = match self.page_images.get(&index) {
                Some(handle) => widget::image(handle.clone())
                    .width(Length::Fill)
                    .height(Length::Fixed(IMAGE_HEIGHT))
                    .content_fit(ContentFit::Contain)
                    .into(),
                None => {
                    let message = self
                        .page_image_errors
                        .get(&index)
                        .map_or("Loading image preview...", String::as_str);

                    widget::container(widget::text(message))
                        .width(Length::Fill)
                        .height(Length::Fixed(IMAGE_HEIGHT))
                        .center_x(Length::Fill)
                        .center_y(Length::Fixed(IMAGE_HEIGHT))
                        .into()
                }
            };
            let caption = widget::text(item.image_caption())
                .width(Length::Fill)
                .size(12)
                .wrapping(cosmic::iced::widget::text::Wrapping::WordOrGlyph);

            widget::button::custom(
                widget::column()
                    .spacing(spacing.space_xxs)
                    .push(header)
                    .push(preview)
                    .push(caption),
            )
            .class(class)
            .selected(selected)
            .width(Length::Fill)
            .padding(spacing.space_s)
            .on_press(Message::SelectIndex(index))
            .into()
        } else {
            let preview = widget::text(item.preview_text())
                .width(Length::Fill)
                .size(14)
                .wrapping(cosmic::iced::widget::text::Wrapping::WordOrGlyph);

            widget::button::custom(
                widget::column()
                    .spacing(spacing.space_xxs)
                    .push(header)
                    .push(widget::container(preview).width(Length::Fill)),
            )
            .class(class)
            .selected(selected)
            .width(Length::Fill)
            .padding(spacing.space_s)
            .on_press(Message::SelectIndex(index))
            .into()
        }
    }

    fn close_window(&self) -> Task<Message> {
        cosmic::iced::exit()
    }

    fn rebuild_filtered(&mut self, preferred_line: Option<&str>) {
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

    fn selected_item(&self) -> Option<&ClipItem> {
        self.selected.and_then(|index| self.items.get(index))
    }

    fn coerce_selection(&mut self, preferred_line: Option<&str>) {
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

    fn sync_page_to_selection(&mut self) {
        let total_pages = page_count(self.filtered.len());

        if total_pages == 0 {
            self.page = 0;
            return;
        }

        self.page = self.page.min(total_pages - 1);

        if let Some(selected) = self.selected
            && let Some(position) = self.filtered.iter().position(|index| *index == selected)
        {
            self.page = position / PAGE_SIZE;
        }
    }

    fn move_selection(&mut self, delta: i32) {
        self.selected = next_selected_index(&self.filtered, self.selected, delta);
    }

    fn change_page(&mut self, delta: isize) -> bool {
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

    fn scroll_to_selection(&self) -> Task<Message> {
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

    fn load_visible_images(&mut self) -> Task<Message> {
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

    fn copy_selected(&mut self) -> Task<Message> {
        let Some(item) = self.selected_item().cloned() else {
            self.status = Some("Nothing is selected.".to_string());
            return Task::none();
        };

        self.status = Some("Copying...".to_string());
        Task::perform(async move { copy_entry(&item) }, |res| {
            cosmic::Action::App(Message::CopyDone(res))
        })
    }

    fn reload_history(&mut self) -> Task<Message> {
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

        Task::batch([
            widget::text_input::focus(self.search_id.clone()),
            self.scroll_to_selection(),
            image_task,
        ])
    }

    fn delete_selected(&mut self) -> Task<Message> {
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
