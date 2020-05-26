#[macro_use]
extern crate num_derive;

pub mod display;
pub mod gui;

use display::monitor::MonitorManager;
use gui::app::App;

fn main() {
    let manager = MonitorManager::new().unwrap();
    let mut application = App::create_with_monitor_manager(manager).unwrap();

    application.run().unwrap();
}
