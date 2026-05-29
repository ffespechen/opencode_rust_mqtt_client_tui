use crate::app::{App, Focus};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Connect,
    Disconnect,
    Subscribe,
    Unsubscribe,
    Publish,
    NextFocus,
    PrevFocus,
    ScrollUp,
    ScrollDown,
    Char(char),
    Backspace,
    Delete,
    Newline,
    CursorLeft,
    CursorRight,
    CursorUp,
    CursorDown,
    Home,
    End,
    ToggleWebSockets,
    ToggleRetain,
    ToggleJson,
    CycleValue,
    TopicUp,
    TopicDown,
    Noop,
}

pub fn key_to_action(key: KeyEvent, app: &App) -> Option<Action> {
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('q')) => Some(Action::Quit),
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => Some(Action::Connect),
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => Some(Action::Disconnect),
        (KeyModifiers::CONTROL, KeyCode::Char('s')) => Some(Action::Subscribe),
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => Some(Action::Unsubscribe),
        (KeyModifiers::CONTROL, KeyCode::Char('p')) => Some(Action::Publish),
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => Some(Action::ToggleWebSockets),
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => Some(Action::ToggleRetain),
        (KeyModifiers::CONTROL, KeyCode::Char('j')) => Some(Action::ToggleJson),
        (KeyModifiers::NONE, KeyCode::Tab) => Some(Action::NextFocus),
        (KeyModifiers::SHIFT, KeyCode::BackTab) => Some(Action::PrevFocus),
        (KeyModifiers::NONE, KeyCode::Char(' ')) => match app.focus {
            Focus::SubscribeQos | Focus::PublishQos => Some(Action::CycleValue),
            _ => Some(Action::Char(' ')),
        },
        (KeyModifiers::NONE, KeyCode::Up) => match app.focus {
            Focus::PublishPayload => Some(Action::CursorUp),
            _ => Some(Action::TopicUp),
        },
        (KeyModifiers::NONE, KeyCode::Down) => match app.focus {
            Focus::PublishPayload => Some(Action::CursorDown),
            _ => Some(Action::TopicDown),
        },
        (KeyModifiers::NONE, KeyCode::Left) => Some(Action::CursorLeft),
        (KeyModifiers::NONE, KeyCode::Right) => Some(Action::CursorRight),
        (KeyModifiers::NONE, KeyCode::Backspace) => Some(Action::Backspace),
        (KeyModifiers::NONE, KeyCode::Delete) => Some(Action::Delete),
        (KeyModifiers::NONE, KeyCode::Enter) => Some(Action::Newline),
        (KeyModifiers::NONE, KeyCode::Home) => Some(Action::Home),
        (KeyModifiers::NONE, KeyCode::End) => Some(Action::End),
        (KeyModifiers::NONE, KeyCode::PageUp) => Some(Action::ScrollUp),
        (KeyModifiers::NONE, KeyCode::PageDown) => Some(Action::ScrollDown),
        (KeyModifiers::NONE, KeyCode::Char(c)) => Some(Action::Char(c)),
        (KeyModifiers::SHIFT, KeyCode::Char(c)) => Some(Action::Char(c.to_ascii_uppercase())),
        _ => Some(Action::Noop),
    }
}
