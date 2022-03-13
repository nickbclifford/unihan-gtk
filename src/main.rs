mod read;

use gtk::gio::Cancellable;
use gtk::glib::{clone, MainContext, Type, PRIORITY_DEFAULT, PRIORITY_HIGH};
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GBox, Button, ButtonsType, CellRendererText,
    DialogFlags, Entry, FileChooserAction, FileChooserNative, FileFilter, ListStore, MessageDialog,
    MessageType, Orientation, ResponseType, TreeView,
};
use rusqlite::types::ValueRef;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub struct InternalError(String);
impl Display for InternalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl<T: Error> From<T> for InternalError {
    fn from(t: T) -> Self {
        InternalError(format!("({}) {}", std::any::type_name::<T>(), t))
    }
}

fn build_ui(app: &Application) {
    let container = GBox::builder().orientation(Orientation::Vertical).build();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("unihan-gtk")
        .child(&container)
        .build();

    let error_dialog = MessageDialog::new(
        Some(&window),
        DialogFlags::DESTROY_WITH_PARENT | DialogFlags::MODAL,
        MessageType::Error,
        ButtonsType::Close,
        "unihan-gtk: internal error",
    );
    let (error_send, error_recv) = MainContext::channel::<InternalError>(PRIORITY_HIGH);
    error_recv.attach(
        None,
        clone!(@strong error_dialog => move |err| {
            error_dialog.set_text(Some(&err.to_string()));
            error_dialog.run_async(|this, _| this.hide());

            Continue(true)
        }),
    );

    macro_rules! spawn {
        ($($inner:tt)*) => {
            let err = error_send.clone();
            std::thread::spawn(move || {
                let expr = || -> Result<(), InternalError> {
                    $($inner)*
                    Ok(())
                };
                if let Err(e) = expr() {
                    err.send(e).unwrap();
                }
            });
        }
    }

    let check_for_field: usize = read::DB
        .lock()
        .unwrap()
        .query_row(
            "SELECT count(name) FROM sqlite_master WHERE type='table' AND name='field'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    if check_for_field == 1 {
        let (snd, rcv) = MainContext::channel(PRIORITY_DEFAULT);
        let output = TreeView::new();
        rcv.attach(
            None,
            clone!(@strong output => move |(cols, data): (Vec<String>, Vec<Vec<String>>)| {
                for col in output.columns() {
                    output.remove_column(&col);
                }
                if cols.is_empty() {
                    output.set_model(None::<&ListStore>);
                    return Continue(true);
                }

                let table = ListStore::new(&vec![Type::STRING; cols.len()]);
                for row in data {
                    let iter = table.append();
                    for (idx, s) in row.iter().enumerate() {
                        table.set_value(&iter, idx as u32, &s.to_value());
                    }
                }
                output.set_model(Some(&table));

                let renderer = CellRendererText::builder().build();
                for (idx, col) in cols.into_iter().enumerate() {
                    output.insert_column_with_attributes(idx as i32, &col, &renderer, &[("text", idx as i32)]);
                }

                Continue(true)
            }),
        );
        container.append(&output);

        let input = Entry::builder().build();
        input.connect_activate(move |this| {
            let val = this.text();
            let sender = snd.clone();
            spawn! {
                let conn = read::DB.lock()?;
                let mut statement = conn.prepare(val.as_str())?;
                let cols: Vec<_> = statement
                            .column_names()
                            .into_iter()
                            .map(String::from)
                            .collect();
                let mut rows = statement.query([])?;

                let mut table_data = Vec::new();
                while let Some(row) = rows.next()? {
                    if table_data.len() > 500 {
                        return Err(InternalError("Query result set too long".to_string()))
                    }
                    let mut cols = vec![];
                    while let Ok(column) = row.get_ref(cols.len()) {
                        cols.push(match column {
                            ValueRef::Null => "NULL".to_string(),
                            ValueRef::Integer(i) => format!("{}", i),
                            ValueRef::Real(f) => format!("{}", f),
                            ValueRef::Text(t) => String::from_utf8(t.to_vec())?,
                            ValueRef::Blob(t) => format!("{:?}", t),
                        });
                    }
                    table_data.push(cols);
                }
                sender
                    .send((
                        cols,
                        table_data,
                    ))?;
            };
            this.set_text("");
        });
        container.append(&input);
    } else {
        let filter = FileFilter::new();
        filter.add_mime_type("application/zip");

        let dialog = FileChooserNative::builder()
            .modal(true)
            .title("Select Unihan database file")
            .action(FileChooserAction::Open)
            .filter(&filter)
            .accept_label("Import")
            .cancel_label("Cancel")
            .transient_for(&window)
            .build();
        dialog.connect_response(move |this, rt| {
            if rt == ResponseType::Accept {
                // Files are Send but the read streams aren't, so move the file
                let file = this.file().unwrap();
                spawn! {
                    read::init_db(file.read(None::<&Cancellable>)?.into_read())?;
                }
            }
        });

        let open_file_button = Button::builder().label("Open database file").build();
        open_file_button.connect_clicked(clone!(@strong dialog => move |_| {
            dialog.show();
        }));
        container.append(&open_file_button);
    }

    window.present();
}

fn main() {
    let app = Application::builder()
        .application_id("me.nickclifford.unihan-gtk")
        .build();

    app.connect_activate(build_ui);

    app.run();
}
