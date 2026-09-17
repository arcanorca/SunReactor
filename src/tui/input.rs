use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::model::Tab;

/// User-facing intent, independent of the physical key used to invoke it.
///
/// Widgets never inspect keyboard events directly. The update layer receives
/// one of these actions and applies it to the focused control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiAction {
    SelectTab(Tab),
    NextTab,
    PreviousTab,
    Quit,
    ToggleHelp,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Activate,
    Back,
    Toggle,
    PageUp,
    PageDown,
    MoveFirst,
    MoveLast,
    NextMonitor,
    PreviousMonitor,
    OpenAutomation,
    Suspend,
    ContextualReset,
    RetryWeather,
}

/// Maps normal-mode keys to a deliberately small semantic action vocabulary.
/// The update reducer decides what an action means for the current focus.
#[must_use]
pub(crate) fn map_normal_key(key: KeyEvent) -> Option<UiAction> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c' | 'C'))
    {
        return Some(UiAction::Quit);
    }

    match key.code {
        KeyCode::Char('1') => Some(UiAction::SelectTab(Tab::Monitors)),
        KeyCode::Char('2') => Some(UiAction::SelectTab(Tab::Limits)),
        KeyCode::Char('3') => Some(UiAction::SelectTab(Tab::Location)),
        KeyCode::Char('4') => Some(UiAction::SelectTab(Tab::Weather)),
        KeyCode::Char('5') => Some(UiAction::SelectTab(Tab::Settings)),
        KeyCode::Char('q') => Some(UiAction::Quit),
        KeyCode::Char('?') | KeyCode::F(1) => Some(UiAction::ToggleHelp),
        // Browser-style workspace keys. Shift+Tab and Ctrl+PgUp/PgDn work in
        // every terminal; Ctrl+Tab and Ctrl+Shift+Tab need a terminal that
        // reports modifiers on Tab (enabled at startup when supported).
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => Some(UiAction::PreviousTab),
        KeyCode::Tab => Some(UiAction::NextTab),
        KeyCode::BackTab => Some(UiAction::PreviousTab),
        KeyCode::PageUp if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(UiAction::PreviousTab)
        }
        KeyCode::PageDown if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(UiAction::NextTab)
        }
        KeyCode::Up | KeyCode::Char('k') => Some(UiAction::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(UiAction::MoveDown),
        KeyCode::Left | KeyCode::Char('h') => Some(UiAction::MoveLeft),
        KeyCode::Right | KeyCode::Char('l') => Some(UiAction::MoveRight),
        KeyCode::Enter => Some(UiAction::Activate),
        KeyCode::Char(' ') => Some(UiAction::Toggle),
        KeyCode::Esc => Some(UiAction::Back),
        KeyCode::PageUp => Some(UiAction::PageUp),
        KeyCode::PageDown => Some(UiAction::PageDown),
        KeyCode::Home => Some(UiAction::MoveFirst),
        KeyCode::End => Some(UiAction::MoveLast),
        // Expert aliases remain available, but the visible selector is the
        // standard way to change monitor.
        KeyCode::Char(']') => Some(UiAction::NextMonitor),
        KeyCode::Char('[') => Some(UiAction::PreviousMonitor),
        KeyCode::Char('a') => Some(UiAction::OpenAutomation),
        KeyCode::Char('s') => Some(UiAction::Suspend),
        KeyCode::Char('r') => Some(UiAction::ContextualReset),
        KeyCode::Char('R') => Some(UiAction::RetryWeather),
        _ => None,
    }
}
