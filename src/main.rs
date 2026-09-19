mod desktop_entry;
pub mod providers;

use desktop_entry::{filter_apps, scan_applications};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};

const APP_ID: &str = "dev.px.rusted-light";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let apps = scan_applications();
    let _matches = filter_apps(&apps, "");

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Rusted Light")
        .default_width(640)
        .default_height(80)
        .build();

    window.present();
}
