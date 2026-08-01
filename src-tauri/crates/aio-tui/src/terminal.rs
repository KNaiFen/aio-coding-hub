use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::sync::atomic::{AtomicBool, Ordering};

static RESTORING: AtomicBool = AtomicBool::new(false);

pub type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

pub struct TerminalSession {
    terminal: TuiTerminal,
}

impl TerminalSession {
    pub fn enter() -> io::Result<Self> {
        install_panic_restore();
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        let result = (|| {
            let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
            terminal.clear()?;
            Ok(Self { terminal })
        })();
        if result.is_err() {
            restore_terminal();
        }
        result
    }

    pub fn terminal_mut(&mut self) -> &mut TuiTerminal {
        &mut self.terminal
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        restore_terminal();
    }
}

fn install_panic_restore() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous(info);
    }));
}

fn restore_terminal() {
    if RESTORING.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
    RESTORING.store(false, Ordering::SeqCst);
}
