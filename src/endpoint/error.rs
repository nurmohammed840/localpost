use std::{error::Error, fmt};

#[derive(Debug)]
pub struct Canceled;

impl Error for Canceled {}
impl fmt::Display for Canceled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the operation was canceled")
    }
}
