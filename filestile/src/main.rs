use std::env;
use std::fs;
use chrono::{DateTime, Utc};

fn main() {


    let path = env::args().nth(1).expect("No path given");

    match fs::read_dir(path) {
        Ok(dir) => {
            for entry in dir {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_dir() {
                            continue;
                        }
                        let modified_nsec = metadata.modified();
                        let created_nsec = metadata.created();

                        if let Ok(modified_nsec) = modified_nsec {
                            let modified: DateTime<Utc> = DateTime::<Utc>::from(modified_nsec);
                            println!("{}: modified @{}", entry.file_name().to_string_lossy(), modified);
                        }

                        if let Ok(created_nsec) = created_nsec {
                            let created: DateTime<Utc> = DateTime::<Utc>::from(created_nsec);
                            println!("{}: created @{}", entry.file_name().to_string_lossy(), created);
                        }

                        // println!("{}", entry.file_name().to_string_lossy());
                        // println!("{:?}: {:?}", entry.path(), metadata.permissions());
                        // println!("{}: {:?}", entry.file_name().to_string_lossy(), metadata.permissions());
                    }

                 }
            }
        }
        Err(err) => println!("Error reading directory: {}", err),
    }
}
