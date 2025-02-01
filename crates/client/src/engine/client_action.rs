use super::input_mode::InputMode;

#[derive(Debug, PartialEq)]
pub enum ClientAction {
    StartComposition,
    EndComposition,

    AppendText(String),
    RemoveText,
    ShrinkText(String),

    MoveCursor(i32),
    SetSelection(SetSelectionType),

    SetIMEMode(InputMode),
}

#[derive(Debug, PartialEq)]
pub enum SetSelectionType {
    Up,
    Down,
    Number(i32),
}
