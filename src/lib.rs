use thiserror::Error;

pub mod btree;
pub mod page;
pub(crate) mod parse;

#[derive(Error, Debug)]
pub enum Error {
    #[error("File error")]
    FileIO(#[from] std::io::Error),
    #[error("Incorrect page type: {0}")]
    PageType(u8),
    #[error("Parse error: {0}")]
    Nom(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;
