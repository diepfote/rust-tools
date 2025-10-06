use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::sync::OnceLock;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use regex::Regex;

mod logging;

struct Args {
    path: String,
    patterns: Vec<OsString>,
    match_group_indexes: Vec<i32>,
}

fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut path = None;
    let mut patterns: Vec<OsString> = Vec::new();
    let mut match_group_indexes: Vec<i32> = Vec::new();
    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('e') | Long("patterns") => {
                patterns = parser.values()?.collect();
            }

            Short('m') | Long("match-group-indexes") => {
                match_group_indexes = parser
                    .values()?
                    .filter_map(|osstr| osstr.to_string_lossy().parse::<i32>().ok())
                    .collect();
            }

            Value(val) if path.is_none() => {
                path = Some(val.string()?);
            }
            Short('h') | Long("help") => {
                println!("Usage: filestile --match-group-indexes --patterns PATTERNS -- PATH");
                println!("Usage: filestile -m INDEXES -e PATTERNS -- PATH");
                println!("e.g.: filestile -m 2 3 -e '.*(Blocksberg|Tina).*(Folge [0-9]+).*'  -- .");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Args {
        patterns: if patterns.is_empty() {
            return Err("missing option -e/--patterns".into());
        } else {
            patterns
        },
        match_group_indexes: if match_group_indexes.is_empty() {
            return Err("missing option -m/--match-group-indexes".into());
        } else {
            match_group_indexes
        },
        path: path.ok_or("missing argument PATH")?,
    })
}

#[derive(Debug)]
struct File {
    name: String,
    ts: SystemTime,
}

fn main() -> Result<(), lexopt::Error> {
    let mut last_matches: HashMap<String, File> = HashMap::new();

    let args = parse_args()?;
    let path_to_search = args.path;
    let match_group_indexes = args.match_group_indexes;

    debug!("path_to_search: {}", path_to_search);
    debug!("patterns: {:?}", args.patterns);

    static RE: OnceLock<Regex> = OnceLock::new();
    let patterns = args
        .patterns
        .iter()
        .map(|os| "(".to_owned() + &os.to_string_lossy() + ")")
        .collect::<Vec<_>>()
        .join("|");
    let re = RE.get_or_init(|| Regex::new(&patterns).unwrap());
    debug!("re: {:?}", re);

    println!("");

    if let Ok(dir) = fs::read_dir(&path_to_search) {
        for entry in dir {
            if let Ok(entry) = entry
                && let Ok(metadata) = entry.metadata()
                && let Ok(created_nsec) = metadata.created()
            {
                if metadata.is_dir() {
                    continue;
                }

                debug!("--------------------------------");

                let p = entry.path().to_string_lossy().to_string();

                if re.is_match(&p) {
                    debug!("Matched: {}", p);
                } else {
                    // debug!("No match: {}\n---", p);
                    continue;
                }

                let _created: DateTime<Utc> = DateTime::<Utc>::from(created_nsec);
                debug!("created @{}", _created);

                let caps = re.captures(&p).unwrap();
                // that all patterns adhere to
                debug!("caps: {:?}", caps);

                let groups = match_group_indexes
                    .iter()
                    .filter_map(|&idx| {
                        caps.get(idx.try_into().unwrap())
                            .map(|match_group| match_group.as_str())
                    })
                    .collect::<Vec<&str>>();
                let shared_fname_section = groups.join(" ");
                // e.g. if a file is: /.../tmp.uaF4y1nMl0/Bibi & Tina -  Das sprechende Pferd (Folge 29) _ Hörspiel des Monats - DAS ZWEITPLATZIERTE....m4a

                //   and the pattern is: '.*(Blocksberg|Tina).*(Folge [0-9]+).*'
                // , then shared_fname_section is: Tina Folge 29

                log_info!("shared_fname_section: {}", shared_fname_section);

                // Remember: we want to keep the oldest file
                if let Some(file) = last_matches.get(&shared_fname_section) {
                    debug!("File already saved.");
                    if file.ts.clone() > created_nsec {
                        log_info!("Current file older, continuing.");
                        log_info!("Removing previous entry: {}", file.name);
                        log_info!("Adding: {}", p);
                        println!("---");
                        last_matches.retain(|key, _| re.is_match(key));
                    } else {
                        debug!("Current file newer, skipping. Keeping: {}", file.name);
                        continue;
                    }
                } else {
                    debug!("None @{}: {}", p, shared_fname_section);
                }

                last_matches.insert(
                    shared_fname_section.to_string(),
                    File {
                        name: p,
                        ts: created_nsec,
                    },
                );
            }
        }
    }
    debug!("last_matches: {:?}", last_matches);

    let keep: HashSet<String> = last_matches
        .values()
        .map(|file| file.name.clone())
        .collect();

    println!("---");

    match fs::read_dir(path_to_search) {
        Ok(dir) => {
            for entry in dir {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_dir() {
                            continue;
                        }

                        let p = entry.path().to_string_lossy().to_string();

                        if !keep.contains(&p) {
                            // @TODO add dry-run
                            let _ = fs::remove_file(&p);
                            log_info!("Deleted {}.", p);
                        }
                    }
                }
            }
        }
        Err(err) => log_err!("Error reading directory: {}", err),
    }

    Ok(())
}
