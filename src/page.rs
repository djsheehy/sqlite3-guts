use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::num::{NonZeroU16, NonZeroU64};
use std::path::Path;

use crate::{parse, Error, Result};

pub struct SqliteFile {
    page_size: u64,
    page1: Page,
    file: RefCell<File>,
}

impl SqliteFile {
    /// Open a database file from a path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<SqliteFile> {
        let mut f = File::open(path)?;
        // get page size at offset 16
        f.seek(SeekFrom::Start(16))?;
        let mut page_size = [0u8, 0];
        f.read(&mut page_size[..])?;
        let page_size = u16::from_be_bytes(page_size) as u64;
        let page_size = if page_size == 1 { 65536 } else { page_size };
        // get first page
        f.seek(SeekFrom::Start(0))?;
        let mut data = vec![0u8; page_size as usize];
        f.read_exact(data.as_mut())?;
        Ok(SqliteFile {
            page_size,
            page1: Page {
                data,
                page_number: 1,
            },
            file: RefCell::new(f),
        })
    }

    pub fn get_page<'a>(&'a self, pagenum: NonZeroU64) -> Result<Page> {
        let pagenum = pagenum.get();
        self.file
            .borrow_mut()
            .seek(SeekFrom::Start((pagenum - 1) * self.page_size))?;
        let mut data = vec![0u8; self.page_size as usize];
        self.file.borrow_mut().read_exact(&mut data)?;
        Ok(Page {
            data,
            page_number: pagenum,
        })
    }
}

pub struct Page {
    data: Vec<u8>,
    pub page_number: u64,
}

impl Page {
    pub fn get_header(&self) -> Result<PageHeader> {
        let input = if self.page_number == 1 {
            &self.data[100..]
        } else {
            &self.data
        };
        Ok(parse::page_header(input)
            .map_err(|_| Error::Nom("page header"))?
            .1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageType {
    InteriorIndex,
    InteriorTable,
    LeafIndex,
    LeafTable,
}

impl TryFrom<u8> for PageType {
    type Error = Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0x02 => Ok(PageType::InteriorIndex),
            0x05 => Ok(PageType::InteriorTable),
            0x0a => Ok(PageType::LeafIndex),
            0x0d => Ok(PageType::LeafTable),
            _ => Err(Error::PageType(value)),
        }
    }
}

/// B-Tree Page Header
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct PageHeader {
    pub page_type: PageType,
    pub first_freeblock: Option<NonZeroU16>,
    pub cell_count: u16,
    pub cell_content: u16,
    pub fragmented_free_bytes: u8,
    pub right_pointer: Option<u32>,
}
