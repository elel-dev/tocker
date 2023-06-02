mod tocker;
mod tui;

use crossterm::terminal::enable_raw_mode;
use std::io;
use tui::Tui;

fn main() -> Result<(), io::Error> {
    enable_raw_mode().unwrap();

    let mut tocker_tui = Tui::new()?;
    tocker_tui.draw_ui().unwrap();

    tocker_tui.start_loop();

    Ok(())
}
