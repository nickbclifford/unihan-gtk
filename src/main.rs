mod read;

use gtk::gio::Cancellable;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Button, FileChooserAction, FileChooserNative, FileFilter,
    ResponseType,
};

fn build_ui(app: &Application) {
    let filter = FileFilter::new();
    filter.add_mime_type("application/zip");

    let open_file_button = Button::builder().label("Open database file").build();

    let dialog = FileChooserNative::builder()
        .modal(true)
        .title("Select Unihan database file")
        .action(FileChooserAction::Open)
        .filter(&filter)
        .accept_label("Open")
        .cancel_label("Cancel")
        .build();

    dialog.connect_response(|this, rt| {
        if rt == ResponseType::Accept {
            // Files are Send but the read streams aren't, so move the file
            let file = this.file().unwrap();

            std::thread::spawn(move || {
                read::init_db(file.read(None::<&Cancellable>).unwrap().into_read()).unwrap();
            });
        }
    });

    open_file_button.connect_clicked(clone!(@strong dialog => move |_| {
        dialog.show();
    }));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("unihan-gtk")
        .child(&open_file_button)
        .build();

    dialog.set_transient_for(Some(&window));

    window.present();
}

fn main() {
    let app = Application::builder()
        .application_id("me.nickclifford.unihan-gtk")
        .build();

    app.connect_activate(build_ui);

    app.run();
}
