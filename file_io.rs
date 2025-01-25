use std::fs::read;
use std::fs::write;

use std::string::FromUtf8Error;

pub fn read_file(filename: &str) -> Result<String, FromUtf8Error> {
    let content = read(filename).unwrap();
    return String::from_utf8(content);
}

pub fn write_file(filename: &str, content: &str) -> () {
    write(filename, content).unwrap();
}
