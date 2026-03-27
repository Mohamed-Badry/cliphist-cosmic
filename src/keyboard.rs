use cosmic::iced::Subscription;
use cosmic::iced::event::wayland::{Event as WaylandEvent, LayerEvent};
use cosmic::iced::event::{self, Event};
use cosmic::iced::keyboard::key::Named;
use cosmic::iced::keyboard::{self, Key};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::messages::{Message, VimAction};

static VIM_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn subscription(vim_enabled: bool) -> Subscription<Message> {
    VIM_ENABLED.store(vim_enabled, Ordering::Relaxed);
    event::listen_with(map_event)
}

fn map_event(
    event: Event,
    status: event::Status,
    _window: cosmic::iced::window::Id,
) -> Option<Message> {
    match event {
        Event::Window(cosmic::iced::window::Event::Unfocused) => Some(Message::CloseWindow),
        Event::PlatformSpecific(event::PlatformSpecific::Wayland(WaylandEvent::Layer(
            LayerEvent::Unfocused,
            _,
            _,
        ))) => Some(Message::CloseWindow),
        Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modified_key,
            modifiers,
            ..
        }) => key_message(
            key.as_ref(),
            modified_key.as_ref(),
            modifiers,
            status,
            current_vim_enabled(),
        ),
        _ => None,
    }
}

fn current_vim_enabled() -> bool {
    VIM_ENABLED.load(Ordering::Relaxed)
}

fn key_message(
    key: Key<&str>,
    modified_key: Key<&str>,
    modifiers: keyboard::Modifiers,
    status: event::Status,
    vim_enabled: bool,
) -> Option<Message> {
    escape_message(&modified_key, status)
        .or_else(|| always_handled_message(&modified_key, modifiers))
        .or_else(|| {
            vim_enabled
                .then(|| vim_message(&modified_key, modifiers))
                .flatten()
        })
        .or_else(|| search_text_message(&modified_key, modifiers, status))
        .or_else(|| ignored_status_message(&key, modifiers, status))
}

fn escape_message(key: &Key<&str>, status: event::Status) -> Option<Message> {
    match key {
        Key::Named(Named::Escape) if status == event::Status::Ignored => {
            Some(Message::GlobalEscape)
        }
        Key::Named(Named::Escape) => None,
        _ => None,
    }
}

fn always_handled_message(key: &Key<&str>, modifiers: keyboard::Modifiers) -> Option<Message> {
    match key {
        Key::Named(Named::ArrowUp) if !has_conflicting_modifiers(modifiers) => {
            Some(Message::MoveSelection(-1))
        }
        Key::Named(Named::ArrowDown) if !has_conflicting_modifiers(modifiers) => {
            Some(Message::MoveSelection(1))
        }
        Key::Named(Named::ArrowLeft) if !has_conflicting_modifiers(modifiers) => {
            Some(Message::PrevPage)
        }
        Key::Named(Named::ArrowRight) if !has_conflicting_modifiers(modifiers) => {
            Some(Message::NextPage)
        }
        Key::Named(Named::Enter) if !has_conflicting_modifiers(modifiers) => {
            Some(Message::ActivateSelection)
        }
        _ => None,
    }
}

fn vim_message(key: &Key<&str>, modifiers: keyboard::Modifiers) -> Option<Message> {
    if has_conflicting_modifiers(modifiers) {
        return None;
    }

    let action = match key {
        Key::Character("/") => Some(VimAction::EnterInsertMode),
        _ => match normalized_char(key) {
            Some('j') => Some(VimAction::MoveDown),
            Some('k') => Some(VimAction::MoveUp),
            Some('h') => Some(VimAction::PrevPage),
            Some('l') => Some(VimAction::NextPage),
            Some('d') => Some(VimAction::DeleteSelected),
            Some('r') => Some(VimAction::Reload),
            Some('i') => Some(VimAction::EnterInsertMode),
            Some('y') => Some(VimAction::ActivateSelection),
            Some('q') => Some(VimAction::CloseWindow),
            _ => None,
        },
    }?;

    Some(Message::HandleVimAction(action))
}

fn search_text_message(
    key: &Key<&str>,
    modifiers: keyboard::Modifiers,
    status: event::Status,
) -> Option<Message> {
    if status != event::Status::Ignored
        || modifiers.control()
        || modifiers.alt()
        || modifiers.logo()
    {
        return None;
    }

    let Key::Character(text) = key else {
        return None;
    };

    if text.chars().any(char::is_control) {
        return None;
    }

    Some(Message::InsertSearchText((*text).to_string()))
}

fn ignored_status_message(
    key: &Key<&str>,
    modifiers: keyboard::Modifiers,
    status: event::Status,
) -> Option<Message> {
    if status != event::Status::Ignored {
        return None;
    }

    if modifiers.control()
        && !modifiers.alt()
        && !modifiers.shift()
        && !modifiers.logo()
        && normalized_char(key) == Some('r')
    {
        return Some(Message::Reload);
    }

    match key {
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
        _ => None,
    }
}

fn normalized_char(key: &Key<&str>) -> Option<char> {
    let Key::Character(text) = key else {
        return None;
    };

    let mut chars = text.chars();
    let ch = chars.next()?;

    if chars.next().is_some() {
        return None;
    }

    Some(ch.to_ascii_lowercase())
}

fn has_conflicting_modifiers(modifiers: keyboard::Modifiers) -> bool {
    modifiers.control() || modifiers.alt() || modifiers.logo()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ignored() -> event::Status {
        event::Status::Ignored
    }

    fn captured() -> event::Status {
        event::Status::Captured
    }

    #[test]
    fn maps_vim_letters_from_logical_keys() {
        assert_eq!(
            vim_message(&Key::Character("j"), keyboard::Modifiers::empty()),
            Some(Message::HandleVimAction(VimAction::MoveDown))
        );
        assert_eq!(
            vim_message(&Key::Character("K"), keyboard::Modifiers::SHIFT),
            Some(Message::HandleVimAction(VimAction::MoveUp))
        );
        assert_eq!(
            vim_message(&Key::Character("q"), keyboard::Modifiers::empty()),
            Some(Message::HandleVimAction(VimAction::CloseWindow))
        );
    }

    #[test]
    fn maps_vim_search_and_page_keys() {
        assert_eq!(
            vim_message(&Key::Character("/"), keyboard::Modifiers::empty()),
            Some(Message::HandleVimAction(VimAction::EnterInsertMode))
        );
        assert_eq!(
            vim_message(&Key::Named(Named::ArrowLeft), keyboard::Modifiers::empty()),
            None
        );
        assert_eq!(
            vim_message(&Key::Named(Named::ArrowRight), keyboard::Modifiers::empty()),
            None
        );
    }

    #[test]
    fn ignores_vim_commands_with_control_modifiers() {
        assert_eq!(
            vim_message(&Key::Character("j"), keyboard::Modifiers::CTRL),
            None
        );
    }

    #[test]
    fn keeps_escape_and_enter_global() {
        assert_eq!(
            key_message(
                Key::Named(Named::Escape),
                Key::Named(Named::Escape),
                keyboard::Modifiers::empty(),
                captured(),
                true,
            ),
            None
        );
        assert_eq!(
            key_message(
                Key::Named(Named::Escape),
                Key::Named(Named::Escape),
                keyboard::Modifiers::empty(),
                ignored(),
                true,
            ),
            Some(Message::GlobalEscape)
        );
        assert_eq!(
            key_message(
                Key::Named(Named::Enter),
                Key::Named(Named::Enter),
                keyboard::Modifiers::empty(),
                captured(),
                true,
            ),
            Some(Message::ActivateSelection)
        );
    }

    #[test]
    fn keeps_left_right_global() {
        assert_eq!(
            key_message(
                Key::Named(Named::ArrowLeft),
                Key::Named(Named::ArrowLeft),
                keyboard::Modifiers::empty(),
                captured(),
                true,
            ),
            Some(Message::PrevPage)
        );
        assert_eq!(
            key_message(
                Key::Named(Named::ArrowRight),
                Key::Named(Named::ArrowRight),
                keyboard::Modifiers::empty(),
                captured(),
                true,
            ),
            Some(Message::NextPage)
        );
    }

    #[test]
    fn ignored_navigation_keys_still_work_while_search_has_focus() {
        assert_eq!(
            key_message(
                Key::Named(Named::PageUp),
                Key::Named(Named::PageUp),
                keyboard::Modifiers::empty(),
                ignored(),
                true,
            ),
            Some(Message::PrevPage)
        );
        assert_eq!(
            key_message(
                Key::Named(Named::PageDown),
                Key::Named(Named::PageDown),
                keyboard::Modifiers::empty(),
                ignored(),
                true,
            ),
            Some(Message::NextPage)
        );
        assert_eq!(
            key_message(
                Key::Named(Named::Home),
                Key::Named(Named::Home),
                keyboard::Modifiers::empty(),
                ignored(),
                true,
            ),
            Some(Message::MoveSelection(i32::MIN))
        );
        assert_eq!(
            key_message(
                Key::Named(Named::End),
                Key::Named(Named::End),
                keyboard::Modifiers::empty(),
                ignored(),
                true,
            ),
            Some(Message::MoveSelection(i32::MAX))
        );
    }

    #[test]
    fn delete_requires_ignored_status_without_extra_modifiers() {
        assert_eq!(
            key_message(
                Key::Named(Named::Delete),
                Key::Named(Named::Delete),
                keyboard::Modifiers::empty(),
                ignored(),
                true,
            ),
            Some(Message::DeleteSelected)
        );
        assert_eq!(
            key_message(
                Key::Named(Named::Delete),
                Key::Named(Named::Delete),
                keyboard::Modifiers::SHIFT,
                ignored(),
                true,
            ),
            None
        );
        assert_eq!(
            key_message(
                Key::Named(Named::Delete),
                Key::Named(Named::Delete),
                keyboard::Modifiers::empty(),
                captured(),
                true,
            ),
            None
        );
    }

    #[test]
    fn reload_shortcut_uses_logical_key_when_event_is_ignored() {
        assert_eq!(
            key_message(
                Key::Character("r"),
                Key::Character("r"),
                keyboard::Modifiers::CTRL,
                ignored(),
                true,
            ),
            Some(Message::Reload)
        );
        assert_eq!(
            key_message(
                Key::Character("r"),
                Key::Character("r"),
                keyboard::Modifiers::CTRL,
                captured(),
                true,
            ),
            None
        );
    }

    #[test]
    fn typing_without_focus_starts_search() {
        assert_eq!(
            key_message(
                Key::Character("a"),
                Key::Character("a"),
                keyboard::Modifiers::empty(),
                ignored(),
                false,
            ),
            Some(Message::InsertSearchText("a".to_string()))
        );
        assert_eq!(
            key_message(
                Key::Character("A"),
                Key::Character("A"),
                keyboard::Modifiers::SHIFT,
                ignored(),
                false,
            ),
            Some(Message::InsertSearchText("A".to_string()))
        );
    }

    #[test]
    fn non_vim_mode_does_not_intercept_vim_letters() {
        assert_eq!(
            key_message(
                Key::Character("j"),
                Key::Character("j"),
                keyboard::Modifiers::empty(),
                ignored(),
                false,
            ),
            Some(Message::InsertSearchText("j".to_string()))
        );
        assert_eq!(
            key_message(
                Key::Character("i"),
                Key::Character("i"),
                keyboard::Modifiers::empty(),
                ignored(),
                false,
            ),
            Some(Message::InsertSearchText("i".to_string()))
        );
    }
}
