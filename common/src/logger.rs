use std::sync::{Mutex, atomic::{AtomicUsize, Ordering}};
use std::io::{stdout, Write};
use log::{Record, Level, Metadata};
use chrono::Local;
use crossterm::terminal::size;

const MAX_BODY_LOGS: usize = 2000;

pub struct LoggerState {
    writer: std::io::Stdout,
    pub header_buffer: Vec<String>,
    body_buffer: Vec<String>,
    is_initialized: bool,
}

pub struct StickyLogger {
    pub state: Mutex<LoggerState>,
    header_height: AtomicUsize,
    body_height: usize,
    show_newest_first: bool,
}

impl StickyLogger {
    pub fn new(header_lines: usize, body_lines: usize, show_newest_first: bool) -> Self {
        let state = LoggerState {
            writer: stdout(),
            header_buffer: Vec::new(),
            body_buffer: Vec::new(),
            is_initialized: false,
        };

        StickyLogger {
            state: Mutex::new(state),
            header_height: AtomicUsize::new(header_lines),
            body_height: body_lines,
            show_newest_first,
        }
    }

    fn initialize_display(&self, state: &mut LoggerState) {
        if !state.is_initialized {
            write!(state.writer, "\x1b[?1049h").unwrap();
            write!(state.writer, "\x1b[?25l").unwrap();
            write!(state.writer, "\x1b[H").unwrap();
            state.writer.flush().unwrap();
            state.is_initialized = true;
        }
    }

    pub fn cleanup(&self, state: &mut LoggerState) {
        write!(state.writer, "\x1b[?25h").unwrap();
        write!(state.writer, "\x1b[?1049l").unwrap();
        state.writer.flush().unwrap();
    }

    fn format_log_message(&self, record: &Record) -> String {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        format!("[{timestamp}] [{:>5}] {}", record.level(), record.args())
    }

    fn redraw(&self, state: &mut LoggerState) {
        self.initialize_display(state);
        
        let mut out = String::with_capacity(1024);
        out.push_str("\x1b[H"); // reset cursor to home position

        // use the current header_height from the AtomicUsize
        let header_height = self.header_height.load(Ordering::Relaxed);
        for i in 0..header_height {
            out.push_str("\x1b[2K"); // Clear current line
            if i < state.header_buffer.len() {
                out.push_str(&state.header_buffer[i]);
            }
            out.push('\n');
        }
        
        // draw a separator line full width
        let (cols, _) = size().unwrap_or((80, 24));
        out.push_str("\x1b[2K");
        out.push_str(&"─".repeat(cols as usize));
        out.push('\n');

        // draw body section.
        let total = state.body_buffer.len();
        let count = self.body_height.min(total);
        let start_idx = total.saturating_sub(count);
        if self.show_newest_first {
            for msg in state.body_buffer[start_idx..].iter().rev().take(self.body_height) {
                out.push_str("\x1b[2K");
                out.push_str(msg);
                out.push('\n');
            }
        } else {
            for msg in state.body_buffer[start_idx..].iter().take(self.body_height) {
                out.push_str("\x1b[2K");
                out.push_str(msg);
                out.push('\n');
            }
        }
        
        state.writer.write_all(out.as_bytes()).unwrap();
        state.writer.flush().unwrap();
    }

    // Change set_header_height to take &self and use the AtomicUsize
    pub fn set_header_height(&self, height: usize) {
        self.header_height.store(height, Ordering::Relaxed);
    }
}

impl log::Log for StickyLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut state = self.state.lock().unwrap();
            // exclude header logs from formatting.
            let msg = if state.header_buffer.len() < self.header_height.load(Ordering::Relaxed) {
                record.args().to_string()
            } else {
                self.format_log_message(record)
            };
    
            if state.header_buffer.len() < self.header_height.load(Ordering::Relaxed) {
                state.header_buffer.push(msg);
            } else {
                // truncate if exceeding limit to avoid memory leakage
                state.body_buffer.push(msg);
                if state.body_buffer.len() > MAX_BODY_LOGS {
                    let len = state.body_buffer.len();
                    let drain_count = MAX_BODY_LOGS.saturating_sub(self.body_height);
                    state.body_buffer.drain(0..(len - drain_count));
                }
            }
            self.redraw(&mut state);
        }
    }
    
    fn flush(&self) {
        let mut state = self.state.lock().unwrap();
        state.writer.flush().unwrap();
    }
}

impl Drop for StickyLogger {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            self.cleanup(&mut state);
        }
    }
}


// LOG message normalization to add period in the end
/// Usage examples:
/// - _info!(raw: format!("Address: {}", "address".to_string())); -> "Address: address"
/// - _info!(raw: "Hello"); -> "Hello"
/// - _info!("Address: {}", "address".to_string()); -> "Address: address."
/// - _info!("Hello"); -> "Hello."
#[macro_export]
macro_rules! _info {
    (raw: $msg:expr) => {{
        ::log::info!("{}", $msg)
    }};
    (raw: $fmt:literal, $($arg:expr),+ $(,)?) => {{
        ::log::info!($fmt, $($arg),+)
    }};
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        let normalized_message = if message.trim_end().ends_with('.') {
            message
        } else {
            message + "."
        };
        ::log::info!("{}", normalized_message);
    }};
}

/// Usage examples:
/// - _error!(raw: format!("Address: {}", "address".to_string())); -> "Address: address"
/// - _error!(raw: "Hello"); -> "Hello"
/// - _error!("Address: {}", "address".to_string()); -> "Address: address."
/// - _error!("Hello"); -> "Hello."
#[macro_export]
macro_rules! _error {
    (raw: $msg:expr) => {{
        ::log::error!("{}", $msg)
    }};
    (raw: $fmt:literal, $($arg:expr),+ $(,)?) => {{
        ::log::error!($fmt, $($arg),+)
    }};
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        let normalized_message = if message.trim_end().ends_with('.') {
            message
        } else {
            message + "."
        };
        ::log::error!("{}", normalized_message);
    }};
}

