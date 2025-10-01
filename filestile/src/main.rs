use std::fs;
use chrono::{DateTime, Utc};

struct Args {
    path: String,
    pattern: String,
}

fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut path = None;
    // @TODO: allow multiple patterns
    let mut pattern = None;
    // @TODO: allow multiple patterns
    // let mut literal = None;
    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            // Short('f') => {
            //     literal = parser.value()?.string()?;
            // }

            Short('e') | Long("pattern") => {
                pattern = Some(parser.value()?.string()?);
            }

            Value(val) if path.is_none() => {
                path = Some(val.string()?);
            }
            Long("help") => {
                println!("Usage: filestile --pattern PATTERN PATH");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Args {
        pattern: pattern.ok_or("missing argument --pattern")?,
        path: path.ok_or("missing argument PATH")?,
    })
}

fn main() -> Result<(), lexopt::Error> {

    let args = parse_args()?;
    let path = args.path;
    // let pattern = args.pattern;

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
       Ok(())
}
