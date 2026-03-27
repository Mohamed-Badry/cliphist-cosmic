use crate::cliphist::ImagePreview;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    NoOp,
    DragWindow,
    SearchChanged(String),
    ClearSearch,
    MoveSelection(i32),
    PrevPage,
    NextPage,
    ActivateSelection,
    SelectAndActivate(usize),
    Reload,
    DeleteSelected,
    CloseWindow,
    EnterNormalMode,
    EnterInsertMode,
    HandleVimAction(VimAction),
    GlobalEscape,
    ToggleMenu,
    WipeHistory,
    CopyDone(Result<(), String>),
    DeleteDone(Result<(), String>),
    WipeDone(Result<(), String>),
    PageImagesLoaded {
        request_id: u64,
        images: Vec<(usize, String, Result<ImagePreview, String>)>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum VimMode {
    Normal,
    Insert { last_j: Option<Instant> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VimAction {
    MoveDown,
    MoveUp,
    PrevPage,
    NextPage,
    DeleteSelected,
    Reload,
    EnterInsertMode,
    ActivateSelection,
    CloseWindow,
}
