use std::error::Error;
use std::io;

pub fn io_error<E>(err: E) -> io::Error
where
    E: Into<Box<Error + Send + Sync>>,
{
    io::Error::new(io::ErrorKind::Other, err)
}
