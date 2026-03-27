use cosmic::Element;
use cosmic::iced::{Color, ContentFit, Length, mouse};
use cosmic::theme::Button as ButtonStyle;
use cosmic::widget;
use cosmic::widget::button;

use crate::app::ClipboardApp;
use crate::config::SurfaceMode;
use crate::messages::{Message, VimMode};
use crate::utils::{current_page_indices, page_count};

use cosmic::widget::popover;

fn launcher_button_style(selected: bool) -> ButtonStyle {
    ButtonStyle::Custom {
        active: Box::new(move |focused, theme| {
            let focused = selected || focused;
            let cosmic = theme.cosmic();
            let rad_s = cosmic.corner_radii.radius_s;
            let on_bg: Color = cosmic.on_bg_color().into();
            let a = if focused {
                button::Catalog::hovered(theme, focused, focused, &ButtonStyle::Text)
            } else {
                button::Catalog::active(theme, focused, focused, &ButtonStyle::Text)
            };
            button::Style {
                border_radius: rad_s.into(),
                outline_width: 0.0,
                text_color: Some(on_bg),
                icon_color: Some(on_bg),
                ..a
            }
        }),
        hovered: Box::new(move |focused, theme| {
            let focused = selected || focused;
            let cosmic = theme.cosmic();
            let rad_s = cosmic.corner_radii.radius_s;
            let on_bg: Color = cosmic.on_bg_color().into();
            let text = button::Catalog::hovered(theme, focused, focused, &ButtonStyle::Text);
            button::Style {
                border_radius: rad_s.into(),
                outline_width: 0.0,
                text_color: Some(on_bg),
                icon_color: Some(on_bg),
                ..text
            }
        }),
        disabled: Box::new(|theme| {
            let rad_s = theme.cosmic().corner_radii.radius_s;
            let text = button::Catalog::disabled(theme, &ButtonStyle::Text);
            button::Style {
                border_radius: rad_s.into(),
                outline_width: 0.0,
                ..text
            }
        }),
        pressed: Box::new(move |focused, theme| {
            let focused = selected || focused;
            let cosmic = theme.cosmic();
            let rad_s = cosmic.corner_radii.radius_s;
            let on_bg: Color = cosmic.on_bg_color().into();
            let text = button::Catalog::pressed(theme, focused, focused, &ButtonStyle::Text);
            button::Style {
                border_radius: rad_s.into(),
                outline_width: 0.0,
                text_color: Some(on_bg),
                icon_color: Some(on_bg),
                ..text
            }
        }),
    }
}

impl ClipboardApp {
    pub fn content_view(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let total_pages = page_count(self.filtered.len(), self.config.page_size);
        let visible = current_page_indices(&self.filtered, self.page, self.config.page_size);
        let mode_badge = self.vim_mode.as_ref().map(mode_label);

        let drag_handle = widget::mouse_area(
            widget::container(
                widget::container(widget::text(""))
                    .width(Length::Fixed(44.0))
                    .height(Length::Fixed(4.0))
                    .class(cosmic::theme::Container::Secondary),
            )
            .width(Length::Fill)
            .height(Length::Fixed(16.0))
            .center_x(Length::Fill)
            .center_y(Length::Fixed(16.0)),
        )
        .on_drag(Message::DragWindow)
        .interaction(mouse::Interaction::Grab);

        let search = widget::text_input::search_input("Search clipboard", &self.search_query)
            .on_input(Message::SearchChanged)
            .on_clear(Message::ClearSearch)
            .on_focus(Message::EnterInsertMode)
            .on_unfocus(Message::EnterNormalMode)
            .id(self.search_id.clone())
            .width(Length::Fill);

        let menu_button: Element<'_, Message> = {
            let btn = widget::button::icon(widget::icon::from_name("view-more-symbolic"))
                .padding(spacing.space_xxs)
                .on_press(Message::ToggleMenu)
                .class(ButtonStyle::Text);

            if self.menu_open {
                popover(btn)
                    .popup(self.menu_popup())
                    .position(popover::Position::Bottom)
                    .on_close(Message::ToggleMenu)
                    .into()
            } else {
                btn.into()
            }
        };

        let search_row = widget::row()
            .spacing(spacing.space_s)
            .align_y(cosmic::iced::Alignment::Center)
            .push(search)
            .push(
                widget::container(
                    widget::button::custom(widget::text(""))
                        .on_press(Message::NoOp)
                        .id(self.dummy_id.clone())
                        .width(Length::Fixed(1.0))
                        .height(Length::Fixed(1.0))
                        .padding(0),
                )
                .width(Length::Fixed(0.0))
                .height(Length::Fixed(0.0)),
            )
            .push(menu_button);

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

        let mut info_row = widget::row().spacing(spacing.space_s);
        info_row = info_row.push(widget::text(results_label).size(13));

        if let Some(label) = mode_badge {
            info_row = info_row.push(mode_badge_chip(label));
        }

        info_row = info_row
            .push(widget::space::horizontal())
            .push(widget::text(page_label).size(13));

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

        let footer = shortcut_footer(self.status.as_deref(), self.vim_mode.is_some());

        let mut root = widget::column().height(Length::Fill);

        if self.config.surface_mode == SurfaceMode::Window {
            root = root.push(drag_handle);
        }

        let content = widget::container(
            root.push(
                widget::column()
                    .height(Length::Fill)
                    .spacing(spacing.space_m)
                    .padding([spacing.space_m, spacing.space_l])
                    .push(search_row)
                    .push(info_row)
                    .push(
                        widget::scrollable(list)
                            .id(self.list_id.clone())
                            .height(Length::Fill)
                            .spacing(spacing.space_xs),
                    )
                    .push(footer),
            ),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .class(cosmic::theme::Container::Background);

        content.into()
    }

    pub fn render_item(&self, index: usize) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let item = &self.items[index];
        let selected = self.selected == Some(index);

        if item.kind.is_image() {
            let preview: Element<'_, Message> = match self.page_images.get(&index) {
                Some(handle) => widget::image(handle.clone())
                    .width(Length::Fill)
                    .height(Length::Fixed(self.config.image_height))
                    .content_fit(ContentFit::Contain)
                    .into(),
                None => {
                    let message = self
                        .page_image_errors
                        .get(&index)
                        .map_or("Loading image preview...", String::as_str);

                    widget::container(widget::text(message))
                        .width(Length::Fill)
                        .height(Length::Fixed(self.config.image_height))
                        .center_x(Length::Fill)
                        .center_y(Length::Fixed(self.config.image_height))
                        .into()
                }
            };

            widget::button::custom(preview)
                .class(launcher_button_style(selected))
                .width(Length::Fill)
                .padding(spacing.space_s)
                .on_press(Message::SelectAndActivate(index))
                .into()
        } else {
            let preview = widget::text(item.preview_text(
                self.config.preview_line_limit,
                self.config.preview_char_limit,
            ))
            .width(Length::Fill)
            .size(14)
            .wrapping(cosmic::iced::widget::text::Wrapping::WordOrGlyph);

            widget::button::custom(widget::container(preview).width(Length::Fill))
                .class(launcher_button_style(selected))
                .width(Length::Fill)
                .padding(spacing.space_s)
                .on_press(Message::SelectAndActivate(index))
                .into()
        }
    }

    fn menu_popup(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();

        let menu_item = |label: &str, message: Message| -> Element<'_, Message> {
            widget::button::custom(widget::text(label.to_string()).size(14).width(Length::Fill))
                .class(ButtonStyle::MenuItem)
                .width(Length::Fill)
                .padding([spacing.space_xs, spacing.space_s])
                .on_press(message)
                .into()
        };

        let wipe_item: Element<'_, Message> = widget::button::custom(
            widget::text("Wipe All History")
                .size(14)
                .width(Length::Fill)
                .class(cosmic::theme::Text::Custom(|t| {
                    cosmic::iced::widget::text::Style {
                        color: Some(t.cosmic().accent_text_color().into()),
                    }
                })),
        )
        .class(ButtonStyle::MenuItem)
        .width(Length::Fill)
        .padding([spacing.space_xs, spacing.space_s])
        .on_press(Message::WipeHistory)
        .into();

        widget::container(
            widget::column()
                .width(Length::Fixed(200.0))
                .spacing(spacing.space_xxxs)
                .padding(spacing.space_xxs)
                .push(menu_item("Reload History", Message::Reload))
                .push(menu_item("Delete Selected", Message::DeleteSelected))
                .push(widget::divider::horizontal::light())
                .push(wipe_item),
        )
        .class(cosmic::theme::Container::Dropdown)
        .into()
    }
}

fn mode_label(mode: &VimMode) -> &'static str {
    match mode {
        VimMode::Normal => "NORMAL",
        VimMode::Insert { .. } => "INSERT",
    }
}

fn mode_badge_chip(label: &str) -> Element<'_, Message> {
    widget::container(widget::text(label).size(12))
        .padding([
            cosmic::theme::spacing().space_xxxs,
            cosmic::theme::spacing().space_xs,
        ])
        .class(cosmic::theme::Container::Secondary)
        .into()
}

fn shortcut_footer<'a>(status: Option<&'a str>, vim_mode: bool) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();
    let shortcuts = if vim_mode {
        vim_shortcuts()
    } else {
        default_shortcuts()
    };

    let shortcuts_row = widget::row()
        .spacing(spacing.space_xxs)
        .extend(
            shortcuts
                .into_iter()
                .map(|(keys, label)| shortcut_chip(keys, label)),
        )
        .wrap()
        .vertical_spacing(spacing.space_xxs);

    let footer = widget::column()
        .spacing(spacing.space_xxs)
        .push_maybe(status.map(status_line))
        .push(shortcuts_row);

    widget::container(footer)
        .width(Length::Fill)
        .padding([spacing.space_xxxs, 0])
        .into()
}

fn shortcut_chip<'a>(keys: &'a str, label: &'a str) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    widget::container(
        widget::row()
            .spacing(spacing.space_xxs)
            .align_y(cosmic::iced::Alignment::Center)
            .push(
                widget::text(keys)
                    .size(11)
                    .class(cosmic::theme::Text::Custom(|t| {
                        cosmic::iced::widget::text::Style {
                            color: Some(t.cosmic().accent_text_color().into()),
                        }
                    })),
            )
            .push(widget::text(label).size(11)),
    )
    .padding([spacing.space_xxxs, spacing.space_xs])
    .class(cosmic::theme::Container::Secondary)
    .into()
}

fn status_line(status: &str) -> Element<'_, Message> {
    widget::text(status)
        .size(12)
        .class(cosmic::theme::Text::Custom(|t| {
            cosmic::iced::widget::text::Style {
                color: Some(t.cosmic().accent_text_color().into()),
            }
        }))
        .into()
}

fn default_shortcuts() -> &'static [(&'static str, &'static str)] {
    &[
        ("Type", "Search"),
        ("↑/↓", "Move"),
        ("←/→ or PgUp/PgDn", "Page"),
        ("Enter", "Copy"),
        ("Delete", "Remove"),
        ("Ctrl+R", "Reload"),
        ("Esc", "Close"),
    ]
}

fn vim_shortcuts() -> &'static [(&'static str, &'static str)] {
    &[
        ("i or /", "Search"),
        ("Esc or jk", "Normal"),
        ("j/k", "Move"),
        ("h/l", "Page"),
        ("y", "Copy"),
        ("d", "Delete"),
        ("r", "Reload"),
        ("q", "Close"),
    ]
}
