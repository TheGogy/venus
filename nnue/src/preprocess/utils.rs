use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use clap::error::{Error, ErrorKind, Result};
use utils::memory::boxed_zeroed;

use crate::arch::{NNUEData, QuantNNUEData, RawNNUEData};

macro_rules! impl_load_write {
    ($t:ty) => {
        impl $t {
            /// Read in NNUE data from a raw file path.
            pub fn load_from_file(path: &PathBuf) -> Result<Box<Self>> {
                let mut file = File::open(path)?;

                let expected = std::mem::size_of::<$t>();
                let actual = file.metadata()?.len() as usize;

                if expected != actual {
                    return Err(Error::raw(
                        ErrorKind::InvalidValue,
                        format!("Error loading NNUE: Expected {expected} bytes, found {actual} bytes!"),
                    ));
                }

                unsafe {
                    let mut data: Box<$t> = boxed_zeroed();
                    let buf = std::slice::from_raw_parts_mut((data.as_mut() as *mut $t).cast::<u8>(), expected);
                    file.read_exact(buf)?;
                    Ok(data)
                }
            }

            /// Write the NNUE data to a raw file.
            pub fn write_to_file(&self, path: &PathBuf) -> Result<()> {
                let mut file = File::create(path)?;
                let len = std::mem::size_of::<$t>();

                unsafe {
                    let buf = std::slice::from_raw_parts((self as *const $t).cast::<u8>(), len);
                    file.write_all(buf)?;
                }

                Ok(())
            }
        }
    };
}

impl_load_write!(NNUEData);
impl_load_write!(RawNNUEData);
impl_load_write!(QuantNNUEData);
