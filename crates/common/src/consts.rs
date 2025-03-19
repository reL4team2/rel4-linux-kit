use sel4::{Word, cap::Endpoint};

/// 通过 [sel4::cap::Endpoint] 发送一次数据最大数量
pub const IPC_DATA_LEN: usize = 120 * 8;

/// 通过 [sel4::cap::Endpoint] 发送数据时，reg 的大小
pub const REG_LEN: usize = size_of::<Word>();

/// 默认的 Page 用来映射的 Slot
/// TODO: 找一个更加合适的位置来放置，防止产生冲突
pub const DEFAULT_PAGE_PLACEHOLDER: u64 = 0;

/// 默认的线程的提示
pub const DEFAULT_THREAD_NOTIFICATION: u64 = 17;

/// 默认的父进程的 Endpoint
pub const DEFAULT_PARENT_EP: Endpoint = Endpoint::from_bits(18);

/// 默认的自身提供服务的 Endpoint
pub const DEFAULT_SERVE_EP: Endpoint = Endpoint::from_bits(19);
