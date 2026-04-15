use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use netwatch::app;
use netwatch::config::NetwatchConfig;
use ratatui::prelude::*;
use std::io;

#[tokio::main]
async fn main() -> Result<()> {
    // Handle CLI flags before entering TUI mode
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("netwatch {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "netwatch {} — real-time network diagnostics in your terminal\n\n\
             USAGE:\n    netwatch [OPTIONS]\n    sudo netwatch       Full mode (health probes + packet capture)\n\n\
             OPTIONS:\n    --generate-config    Write a default config file and exit\n    \
             -h, --help           Print help\n    -V, --version        Print version\n\n\
             KEYS (in TUI):\n    1-7   Switch tabs    /     Filter    q   Quit\n    \
             Shift+R/F/E   Flight Recorder: arm / freeze / export",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }
    if args.iter().any(|a| a == "--generate-config") {
        let cfg = NetwatchConfig::default();
        cfg.save()?;
        match NetwatchConfig::path() {
            Some(path) => println!("Config written to {}", path.display()),
            None => println!("Config written (could not determine path)"),
        }
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = app::run(&mut terminal).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e:?}");
    }

    Ok(())
}
