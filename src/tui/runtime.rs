use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    cursor::Show,
    event::{
        self, Event, KeyEventKind, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use super::worker::IpcCommand;
use super::{ui, update, DaemonConnection, Model};

struct TerminalGuard {
    keyboard_enhanced: bool,
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.keyboard_enhanced {
            let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
        }
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    }
}

pub fn run() -> Result<String, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    // Terminals speaking the kitty keyboard protocol can report Ctrl+Tab and
    // Ctrl+Shift+Tab; others keep their usual keys.
    let keyboard_enhanced = matches!(
        crossterm::terminal::supports_keyboard_enhancement(),
        Ok(true)
    ) && execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )
    .is_ok();
    let guard = TerminalGuard { keyboard_enhanced };

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = Model::new();
    let result = run_app(&mut terminal, &mut app);

    let _ = app.ipc_tx.try_send(IpcCommand::Shutdown);

    drop(guard);

    if let Err(error) = result {
        println!("{error:?}");
    }

    Ok(String::from("TUI exited successfully."))
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut Model) -> io::Result<()> {
    app.update_cell_metrics();
    let mut redraw_requested = true;
    let mut last_static_draw = Instant::now();

    loop {
        let now = Instant::now();
        // Pending daemon requests animate a spinner; everything else animates
        // only while a bounded transient is running.
        let pending = matches!(app.action_state, super::model::ActionState::Pending { .. })
            && app.motion.level != crate::config::MotionLevel::Off;
        let animating = app.motion.needs_animation_frame(now) || pending;
        if redraw_requested
            || animating
            || now.duration_since(last_static_draw) >= Duration::from_secs(1)
        {
            terminal.draw(|frame| ui::ui(frame, app))?;
            redraw_requested = false;
            if !animating {
                last_static_draw = now;
            }
        }

        if app.should_quit {
            return Ok(());
        }

        // Transients are short, so they get a smooth cadence regardless of the
        // idle refresh preference; idle frames stay at one per second.
        let fps = app.config.tui.fps.clamp(24, 60);
        let poll_timeout = if app.daemon_connection == DaemonConnection::Unknown {
            Duration::from_millis(10) // fast poll until connected
        } else if animating {
            Duration::from_millis(1000 / u64::from(fps))
        } else {
            // Keep debounce/timeout handling responsive without repainting static frames.
            Duration::from_millis(100)
        };

        if event::poll(poll_timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Release {
                        update::update(app, update::Message::Key(key));
                        redraw_requested = true;
                    }
                }
                Event::Resize(cols, rows) => {
                    update::update(app, update::Message::Resize(cols, rows));
                    redraw_requested = true;
                }
                _ => {}
            }

            while event::poll(Duration::from_millis(0))? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind != KeyEventKind::Release {
                            update::update(app, update::Message::Key(key));
                            redraw_requested = true;
                        }
                    }
                    Event::Resize(cols, rows) => {
                        update::update(app, update::Message::Resize(cols, rows));
                        redraw_requested = true;
                    }
                    _ => {}
                }
            }
        }

        while let Ok(ipc_event) = app.ipc_rx.try_recv() {
            update::update(app, update::Message::Ipc(ipc_event));
            redraw_requested = true;
        }

        let dirty_before_tick = app.config_dirty;
        let action_before_tick = app.action_state.clone();
        update::update(app, update::Message::Tick);
        if dirty_before_tick != app.config_dirty || action_before_tick != app.action_state {
            redraw_requested = true;
        }
    }
}
