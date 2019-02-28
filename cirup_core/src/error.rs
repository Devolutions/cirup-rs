use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum CirupError {
    Io(io::Error),
}

impl fmt::Display for CirupError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CirupError::Io(ref err) => write!(f, "IO error: {}", err),
        }
    }
}

impl error::Error for CirupError {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            CirupError::Io(ref err) => Some(err),
        }
    }
}
