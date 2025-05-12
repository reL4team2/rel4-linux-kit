//! 宏定义

/// 在服务或任务中声明完成初始化后的程序入口
#[macro_export]
macro_rules! entry_point {
    ($main:ident) => {
        #[unsafe(no_mangle)]
        extern "Rust" fn _impl_main() -> ! {
            $main()
        }
    };
}
