use crate::configuration::Settings;
use crate::game::app::App;
use crate::game::tui;

pub fn play(_config: &Settings) -> color_eyre::Result<()> {
    tui::install_hooks()?;
    let mut terminal = tui::init()?;
    let _ = App::default().run(&mut terminal);
    tui::restore()?;
    Ok(())
}
