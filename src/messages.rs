use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    NoOp,
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
    CopyDone(Result<(), String>),
    PageImagesLoaded {
        request_id: u64,
        images: Vec<(usize, Result<Vec<u8>, String>)>,
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
