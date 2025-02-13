use std::sync::Mutex;
use std::io::{stdout, Write};
use log::{Record, Level, Metadata};
use chrono::Local;
use crossterm::terminal::size;

// TODO: styling the log costs us performance, might optimize later by adding option no show log
// for now, it's optimized by building all logs in a single string before output
// without truncating the buffer vector directly only certain limit `MAX_BODY_LOGS`

const MAX_BODY_LOGS: usize = 2000;

pub struct LoggerState {
    writer: std::io::Stdout,
    pub header_buffer: Vec<String>,
    body_buffer: Vec<String>,
    is_initialized: bool,
}

pub struct StickyLogger {
    pub state: Mutex<LoggerState>,
    header_lines: usize,
    body_lines: usize,
    show_newest_first: bool, // true: newest-first, false: oldest-first
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
            header_lines,
            body_lines,
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

        // draw header section.
        for i in 0..self.header_lines {
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
        let count = self.body_lines.min(total);
        let start_idx = total.saturating_sub(count);
        if self.show_newest_first {
            for msg in state.body_buffer[start_idx..].iter().rev().take(self.body_lines) {
                out.push_str("\x1b[2K");
                out.push_str(msg);
                out.push('\n');
            }
        } else {
            for msg in state.body_buffer[start_idx..].iter().take(self.body_lines) {
                out.push_str("\x1b[2K");
                out.push_str(msg);
                out.push('\n');
            }
        }
        
        state.writer.write_all(out.as_bytes()).unwrap();
        state.writer.flush().unwrap();
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
            let msg = if state.header_buffer.len() < self.header_lines {
                record.args().to_string()
            } else {
                self.format_log_message(record)
            };
    
            if state.header_buffer.len() < self.header_lines {
                state.header_buffer.push(msg);
            } else {
                // trucate if exceeding limit to avoid memory leakage
                state.body_buffer.push(msg);
                if state.body_buffer.len() > MAX_BODY_LOGS {
                    let len = state.body_buffer.len();
                    let drain_count = MAX_BODY_LOGS.saturating_sub(self.body_lines);
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
