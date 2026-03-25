use cosmic::Element;
use cosmic::iced::{ContentFit, Length};
use cosmic::theme::Button as ButtonStyle;
use cosmic::widget;

use crate::app::ClipboardApp;
use crate::config::IMAGE_HEIGHT;
use crate::messages::{Message, VimMode};
use crate::utils::{current_page_indices, page_count};

impl ClipboardApp {
    pub fn content_view(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let total_pages = page_count(self.filtered.len());
        let visible = current_page_indices(&self.filtered, self.page);
        let mode_badge = self.vim_mode.as_ref().map(|mode| match mode {
            VimMode::Normal => "[NORMAL]",
            VimMode::Insert { .. } => "[INSERT]",
        });

        let search = widget::text_input::search_input("Search clipboard", &self.search_query)
            .on_input(Message::SearchChanged)
            .on_clear(Message::ClearSearch)
            .on_focus(Message::EnterInsertMode)
            .on_unfocus(Message::EnterNormalMode)
            .id(self.search_id.clone())
            .width(Length::Fill);

        let search_row = {
            let mut row = widget::row().spacing(spacing.space_s).push(search);

            if let Some(label) = mode_badge {
                row = row.push(
                    widget::container(widget::text(label).size(12))
                        .padding([spacing.space_xxs, spacing.space_s])
                        .class(cosmic::theme::Container::Secondary),
                );
            }

            row.push(
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
        };

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

        let status_text = self.status.clone().unwrap_or_else(|| {
            if self.vim_mode.is_some() {
                "j/k move | h/l or Left/Right page | i or / focuses search | Esc leaves Insert or closes"
                    .to_string()
            } else {
                "Enter copies | Page Up/Down switches page | Esc closes".to_string()
            }
        });

        let content = widget::container(
            widget::column()
                .spacing(spacing.space_m)
                .padding([spacing.space_m, spacing.space_l])
                .push(search_row)
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
                .push(widget::text(status_text).size(12)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .class(cosmic::theme::Container::Background);

        widget::layer_container(content).into()
    }

    pub fn render_item(&self, index: usize) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let item = &self.items[index];
        let selected = self.selected == Some(index);

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

            widget::button::custom(preview)
                .class(ButtonStyle::ListItem)
                .selected(selected)
                .width(Length::Fill)
                .padding(spacing.space_s)
                .on_press(Message::SelectAndActivate(index))
                .into()
        } else {
            let preview = widget::text(item.preview_text())
                .width(Length::Fill)
                .size(14)
                .wrapping(cosmic::iced::widget::text::Wrapping::WordOrGlyph);

            widget::button::custom(widget::container(preview).width(Length::Fill))
                .class(ButtonStyle::ListItem)
                .selected(selected)
                .width(Length::Fill)
                .padding(spacing.space_s)
                .on_press(Message::SelectAndActivate(index))
                .into()
        }
    }
}
