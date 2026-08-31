use std::env;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};
use chrono::Local;

// A thread-safe, lazily initialized global variable for our log output.
// Box<dyn Write + Send> allows us to fallback to stderr if the file fails to open.
static LOG_FILE: OnceLock<Mutex<Box<dyn Write + Send>>> = OnceLock::new();

fn get_log_file() -> &'static Mutex<Box<dyn Write + Send>> {
    LOG_FILE.get_or_init(|| {
        // Resolve the process name
        let prog_name = env::current_exe()
            .ok()
            .and_then(|pb| pb.file_name().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "unknown_process".to_string());

        let path = format!("/var/tmp/{}.log", prog_name);

        // Open in append mode, create if it doesn't exist
        match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(file) => Mutex::new(Box::new(file)),
            Err(e) => {
                eprintln!("Failed to open log file {}: {}", path, e);
                // Fallback to stderr if we lack permissions
                Mutex::new(Box::new(io::stderr()))
            }
        }
    })
}

/// Internal function called by the macro.
#[doc(hidden)]
pub fn _log_impl(args: std::fmt::Arguments) {
    // Acquire the lock for thread-safe writing
    let mut writer = get_log_file().lock().unwrap();
    
    // Generate timestamp
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

    // Write the timestamp, the formatted message, and automatically append a newline
    let _ = writeln!(writer, "[{}] {}", timestamp, args);
    
    // Flush immediately so logs aren't lost if the device loses power or panics
    let _ = writer.flush();
}

/// The drop-in replacement for `println!`
#[macro_export]
macro_rules! iotlogger {
    ($($arg:tt)*) => {
        // $crate allows this macro to work anywhere in your project
        $crate::log::_log_impl(format_args!($($arg)*));
    };
}
