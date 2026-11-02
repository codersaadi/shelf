mod apps;
mod dbus;
mod error;
mod fsops;
mod icons;
mod origin;
mod terminal;
mod trash;

pub use apps::{
    AppInfo, apps_for_mime, apps_for_path, default_mime_app, mime_type, open_with, set_default_for,
    set_default_mime,
};
pub use dbus::{open_path, show_in_file_manager};
pub use error::{Error, Result};
pub use fsops::{copy_paths, move_paths};
pub use icons::app_icon_path;
pub use origin::{
    ORIGIN_APP_KEYS, ORIGIN_URL_KEYS, Origin, TAGS_KEY, read_origin, read_tags_xattr,
    write_tags_xattr,
};
pub use terminal::open_terminal;
pub use trash::{
    TrashEntry, Trashed, is_trash_file_path, list_trash, purge_trash_paths, restore_receipt,
    restore_trash_entry, to_trash, to_trash_many,
};
