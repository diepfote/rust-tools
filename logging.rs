// Based on Zed A. Shaw's log functions
// https://learncodethehardway.com/courses/learn-c-the-hard-way/

#[cfg(debug)]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        eprintln!("[DEBUG]: {}", format_args!($($arg)*));
    }};
}
#[cfg(not(debug))]
#[macro_export]
macro_rules! debug {
    ($($args: tt)*) => {
    }
}
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {{
        eprintln!("[INFO]: {}", format_args!($($arg)*));
    }};
}
#[macro_export]
macro_rules! log_err {
    ($($arg:tt)*) => {{
        eprintln!("[INFO]: {}", format_args!($($arg)*));
    }};
}
