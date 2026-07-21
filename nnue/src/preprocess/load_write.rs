use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use clap::error::{Error, ErrorKind, Result};
use utils::memory::boxed_zeroed;

use crate::arch::{NNUEData, QuantNNUEData, RawNNUEData};

pub trait LoadWrite: Sized {
    /// Load the given type from some file.
    ///
    /// # Errors
    ///     Errors when the file cannot be opened, or does not match the file size.
    fn load_from_file(path: &Path) -> Result<Box<Self>> {
        let mut file = File::open(path)?;

        let expected = size_of::<Self>();
        let actual = file.metadata()?.len();

        if (expected as u64) != actual {
            return Err(Error::raw(
                ErrorKind::InvalidValue,
                format!("Error loading {}: Expected {expected} bytes, found {actual} bytes!", std::any::type_name::<Self>()),
            ));
        }

        unsafe {
            let mut data: Box<Self> = boxed_zeroed();
            let buf = std::slice::from_raw_parts_mut(std::ptr::from_mut::<Self>(data.as_mut()).cast::<u8>(), expected);
            file.read_exact(buf)?;
            Ok(data)
        }
    }

    /// Write the given type to some file.
    ///
    /// # Errors
    ///     Errors when the file cannot be written to.
    fn write_to_file(&self, path: &Path) -> Result<()> {
        let mut file = File::create(path)?;
        let len = size_of::<Self>();

        unsafe {
            let buf = std::slice::from_raw_parts(std::ptr::from_ref::<Self>(self).cast::<u8>(), len);
            file.write_all(buf)?;
        }

        Ok(())
    }
}

impl LoadWrite for NNUEData {}
impl LoadWrite for RawNNUEData {}
impl LoadWrite for QuantNNUEData {}
