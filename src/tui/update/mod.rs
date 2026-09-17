use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::input::map_normal_key;
use super::model::ActiveModal;
use super::worker::IpcEvent;
use super::{InputMode, Model};

mod editing;
mod ipc;
mod navigation;
mod normal;

pub(crate) enum Message {
    Key(KeyEvent),
    Ipc(IpcEvent),
    Resize(u16, u16),
    Tick,
}

pub(crate) fn update(model: &mut Model, msg: Message) {
    match msg {
        Message::Key(key) => handle_key(key, model),
        Message::Ipc(event) => ipc::handle_ipc(event, model),
        Message::Resize(cols, rows) => model.handle_resize(cols, rows),
        Message::Tick => {
            model.check_action_state_expiration();
            model.check_debounced_save();
            model.poll_preview_refresh();
            model.refresh_monitor_milestones_if_needed();
            model.motion.tick(std::time::Instant::now());
        }
    }
}

fn handle_key(key: KeyEvent, app: &mut Model) {
    if key.kind == crossterm::event::KeyEventKind::Release {
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c' | 'C'))
    {
        if app.config_dirty && !app.save_config() {
            return;
        }
        app.should_quit = true;
        return;
    }

    if !matches!(app.active_modal, ActiveModal::None) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.theme_modal_up(),
            KeyCode::Down | KeyCode::Char('j') => app.theme_modal_down(),
            KeyCode::Enter => app.theme_modal_confirm(),
            KeyCode::Esc => app.theme_modal_cancel(),
            _ => {}
        }
        return;
    }

    if app.show_help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?' | 'q') => {
                app.show_help = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.scroll_help_up(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.scroll_help_down(1, 24);
            }
            KeyCode::PageUp => {
                app.scroll_help_up(6);
            }
            KeyCode::PageDown => {
                app.scroll_help_down(6, 24);
            }
            KeyCode::Home => {
                app.scroll_help_home();
            }
            KeyCode::End => {
                app.scroll_help_end(24);
            }
            _ => {}
        }
        return;
    }

    match app.input_mode {
        InputMode::Normal => {
            if let Some(action) = map_normal_key(key) {
                normal::dispatch_normal_action(action, app);
            }
        }
        InputMode::Editing => {
            editing::handle_editing_key(key, app);
        }
    }
}
