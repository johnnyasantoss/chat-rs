use std::result::Result as StdResult;
use std::fmt::{Debug, Error as StdError, Formatter};

pub type Result<T> = StdResult<T, Error>;

pub struct Error {
    message: String,
}

impl Debug for Error {
    fn fmt(&self, formatter: &mut Formatter) -> StdResult<(), StdError> {
        formatter.write_str(&format!("Testing {:?}", self))
    }
}
