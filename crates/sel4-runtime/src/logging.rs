//! 日志管理程序
//!
//!

/// 初始化 log，设置 log 级别
#[macro_export]
macro_rules! init_log {
    ($level:expr) => {
        pub fn fmt_with_module(
            record: &log::Record,
            f: &mut core::fmt::Formatter,
        ) -> core::fmt::Result {
            let color_code = match record.level() {
                log::Level::Error => 31u8, // Red
                log::Level::Warn => 93,    // BrightYellow
                log::Level::Info => 34,    // Blue
                log::Level::Debug => 32,   // Green
                log::Level::Trace => 90,   // BrightBlack
            };

            write!(
                f,
                "\u{1B}[{}m\
                [{} {}] {}\
                    \u{1B}[0m",
                color_code,
                env!("CARGO_PKG_NAME"),
                record.level(),
                record.args()
            )
        }
        static LOGGER: $crate::Logger = $crate::LoggerBuilder::const_default()
            .write(|s| sel4::debug_print!("{}", s))
            .level_filter($level)
            .fmt(fmt_with_module)
            .build();
        LOGGER.set().unwrap();
    };
}
