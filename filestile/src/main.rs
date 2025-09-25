use std::env;
use std::fs;

fn main() {


    let path = env::args().nth(1).expect("No path given");

    match fs::read_dir(path) {
        Ok(dir) => {
            for entry in dir {
                match entry {
                    Ok(entry) => println!("{}", entry.file_name().to_string_lossy()),
                    Err(err) => println!("Error: {}", err),
                }
            }
        }
        Err(err) => println!("Error reading directory: {}", err),
    }
}
