use std::error::Error;
use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use super::worker::IpcCommand;
use super::{ui, update, DaemonConnection, Model};

pub fn run() -> Result<String, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = Model::new();
    let result = run_app(&mut terminal, &mut app);

    let _ = app.ipc_tx.try_send(IpcCommand::Shutdown);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(error) = result {
        println!("{error:?}");
    }

    Ok(String::from("TUI exited successfully."))
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut Model) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::ui(frame, app))?;

        if app.should_quit {
            return Ok(());
        }

        let fps = app.config.tui.fps.clamp(1, 120);
        let poll_ms = if app.daemon_connection == DaemonConnection::Unknown {
            10 // fast poll until connected
        } else {
            1000 / u64::from(fps)
        };

        if event::poll(Duration::from_millis(poll_ms))? {
            if let Event::Key(key) = event::read()? {
                update::update(app, update::Message::Key(key));
            }

            while event::poll(Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()? {
                    update::update(app, update::Message::Key(key));
                }
            }
        }

        while let Ok(ipc_event) = app.ipc_rx.try_recv() {
            update::update(app, update::Message::Ipc(ipc_event));
        }

        update::update(app, update::Message::Tick);
    }
}
