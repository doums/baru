// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::Error as IoError;
use std::num::TryFromIntError;
use std::num::{ParseFloatError, ParseIntError};
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::sync::mpsc::{RecvError, SendError};

#[derive(Debug, Clone)]
pub struct Error(String);

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl StdError for Error {}

impl Error {
    pub fn new(item: impl Into<String>) -> Error {
        Error(item.into())
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Error(error)
    }
}

impl From<IoError> for Error {
    fn from(error: IoError) -> Self {
        Error(error.to_string())
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(error: SendError<T>) -> Self {
        Error(error.to_string())
    }
}

impl From<RecvError> for Error {
    fn from(error: RecvError) -> Self {
        Error(error.to_string())
    }
}

impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Error(error.to_string())
    }
}

impl From<Utf8Error> for Error {
    fn from(error: Utf8Error) -> Self {
        Error(error.to_string())
    }
}

impl From<FromUtf8Error> for Error {
    fn from(error: FromUtf8Error) -> Self {
        Error(error.to_string())
    }
}

impl From<ParseIntError> for Error {
    fn from(error: ParseIntError) -> Self {
        Error(error.to_string())
    }
}

impl From<ParseFloatError> for Error {
    fn from(error: ParseFloatError) -> Self {
        Error(error.to_string())
    }
}

impl From<TryFromIntError> for Error {
    fn from(error: TryFromIntError) -> Self {
        Error(error.to_string())
    }
}
