use alloc::fmt;
use log::{Level, LevelFilter, Record};
use sel4::debug_println;
use sel4_logging::Logger;

static mut LOGGER: Logger = sel4_logging::LoggerBuilder::const_default()
    .write(|s| sel4::debug_print!("{}", s))
    .fmt(fmt_with_module)
    .build();

pub fn init() {
    unsafe {
        LOGGER.level_filter = match option_env!("LOG") {
            Some("error") => LevelFilter::Error,
            Some("warn") => LevelFilter::Warn,
            Some("info") => LevelFilter::Info,
            Some("debug") => LevelFilter::Debug,
            Some("trace") => LevelFilter::Trace,
            _ => LevelFilter::Debug,
        };
        LOGGER.set().unwrap();
        debug_println!();
        debug_println!("[Kernel Thread] Log Filter: {:?}", LOGGER.level_filter());
    }
}

pub fn fmt_with_module(record: &Record, f: &mut fmt::Formatter) -> fmt::Result {
    let target = match record.target().is_empty() {
        true => record.module_path().unwrap_or_default(),
        false => record.target(),
    };
    let color_code = match record.level() {
        Level::Error => 31u8, // Red
        Level::Warn => 93,    // BrightYellow
        Level::Info => 34,    // Blue
        Level::Debug => 32,   // Green
        Level::Trace => 90,   // BrightBlack
    };

    write!(
        f,
        "\u{1B}[{}m\
            [{}] [{}] {}\
            \u{1B}[0m",
        color_code,
        record.level(),
        target,
        record.args()
    )
}
