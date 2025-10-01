use std::fs;
use std::ffi::OsString;
use chrono::{DateTime, Utc};
use regex::Regex;
use std::sync::OnceLock;

mod logging;

struct Args {
    path: String,
    patterns: Vec<OsString>,

}

fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut path = None;
    let mut patterns: Vec<OsString> = Vec::new();
    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
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


// [{name: timestamp}, {name: timestamp}]
struct File {
    name: String,
    timestamp: i64,
}


fn main() -> Result<(), lexopt::Error> {

    let args = parse_args()?;
    let path = args.path;

    debug!("path: {}", path);
    debug!("patterns: {:?}", args.patterns);

    static RE: OnceLock<Regex> = OnceLock::new();
    let patterns = args.patterns
        .iter()
        .map(|os| "(".to_owned() + &os.to_string_lossy() + ")")
        .collect::<Vec<_>>()
        .join("|");
    let re = RE.get_or_init(|| Regex::new(&patterns).unwrap());
    debug!("re: {:?}", re);

    println!("");

    match fs::read_dir(path) {
        Ok(dir) => {
            for entry in dir {
                if let Ok(entry) = entry {

                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_dir() {
                            continue;
                        }

                    let p = entry.path().to_string_lossy().to_string();

                    if re.is_match(&p) {
                        log_info!("Matched: {}", p);
                    } else {
                        debug!("No match: {}\n---", p);
                        continue;
                    }

                    let modified_nsec = metadata.modified();
                    let created_nsec = metadata.created();

                    if let Ok(modified_nsec) = modified_nsec {
                        let _modified: DateTime<Utc> = DateTime::<Utc>::from(modified_nsec);
                        debug!("{}: modified @{}", entry.file_name().to_string_lossy(), _modified);
                    }

                    if let Ok(created_nsec) = created_nsec {
                        let _created: DateTime<Utc> = DateTime::<Utc>::from(created_nsec);
                        debug!("{}: created @{}", entry.file_name().to_string_lossy(), _created);
                    }

                    println!("---");
                    }
                 }
            }
        }
        Err(err) => log_err!("Error reading directory: {}", err),

    }
       Ok(())
}
