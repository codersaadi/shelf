use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("trash operation failed: {0}")]
    Trash(#[from] trash::Error),
    #[error("i/o operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("dbus call failed: {0}")]
    Dbus(#[from] zbus::Error),
    #[error("no trash entry found for {0}")]
    NotInTrash(String),
    #[error("no application registered as {0}")]
    NoApp(String),
    #[error("no terminal emulator found")]
    NoTerminal,
}
