use std::fs;
use std::ffi::OsString;
use chrono::{DateTime, Utc};

mod logging;

struct Args {
    path: String,
    patterns: Vec<OsString>,

}

fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut path = None;
    // @TODO: allow multiple patterns
    let mut patterns: Vec<OsString> = Vec::new();
    // @TODO: allow multiple patterns
    // let mut literal = None;
    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            // Short('f') => {
            //     literal = parser.value()?.string()?;
            // }

            Short('e') | Long("patterns") => {
                patterns = parser.values()?.collect();
            }

            Value(val) if path.is_none() => {
                path = Some(val.string()?);
            }
            Long("help") => {
                debug!("Usage: filestile --patterns PATTERNS PATH");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Args {
        patterns: if patterns.is_empty() {
            return Err("missing option --patterns".into());
        } else {
            patterns
        },
        path: path.ok_or("missing argument PATH")?,
    })
}

fn main() -> Result<(), lexopt::Error> {

    let args = parse_args()?;
    let path = args.path;

    debug!("path: {}", path);
    debug!("patterns: {:?}", args.patterns);
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
                            debug!("{}: modified @{}", entry.file_name().to_string_lossy(), modified);
                        }

                        if let Ok(created_nsec) = created_nsec {
                            let created: DateTime<Utc> = DateTime::<Utc>::from(created_nsec);
                            debug!("{}: created @{}", entry.file_name().to_string_lossy(), created);
                        }

                        // debug!("{}", entry.file_name().to_string_lossy());
                        // debug!("{:?}: {:?}", entry.path(), metadata.permissions());
                        // debug!("{}: {:?}", entry.file_name().to_string_lossy(), metadata.permissions());
                    }

                 }
            }
        }
        Err(err) => log_err!("Error reading directory: {}", err),

    }
       Ok(())
}
