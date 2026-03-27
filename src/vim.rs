use cosmic::Application;
use cosmic::app::Task;

use crate::app::ClipboardApp;
use crate::messages::{Message, SelectionMove, VimAction, VimMode};

impl ClipboardApp {
    pub fn handle_vim_action(&mut self, action: VimAction) -> Task<Message> {
        if let Some(VimMode::Normal) = self.vim_mode {
            match action {
                VimAction::MoveDown => {
                    return self.update(Message::MoveSelection(SelectionMove::Relative(1)));
                }
                VimAction::MoveUp => {
                    return self.update(Message::MoveSelection(SelectionMove::Relative(-1)));
                }
                VimAction::PrevPage => return self.update(Message::PrevPage),
                VimAction::NextPage => return self.update(Message::NextPage),
                VimAction::DeleteSelected => return self.update(Message::DeleteSelected),
                VimAction::Reload => return self.update(Message::Reload),
                VimAction::EnterInsertMode => return self.update(Message::EnterInsertMode),
                VimAction::ActivateSelection => {
                    return self.update(Message::ActivateSelection);
                }
                VimAction::CloseWindow => return self.update(Message::CloseWindow),
            }
        }
        Task::none()
    }
}
