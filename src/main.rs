use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Label};

fn build_ui(app: &Application) {
    let label = Label::builder()
        .label("Hello, world!")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("unihan-gtk")
        .child(&label)
        .build();

    window.present();
}

fn main() {
    let app = Application::builder()
        .application_id("me.nickclifford.unihan-gtk")
        .build();

    app.connect_activate(build_ui);

    app.run();
}
