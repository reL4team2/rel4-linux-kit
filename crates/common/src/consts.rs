use sel4::Word;

/// 通过 [sel4::cap::Endpoint] 发送一次数据最大数量
pub const IPC_DATA_LEN: usize = 120 * 8;

/// 通过 [sel4::cap::Endpoint] 发送数据时，reg 的大小
pub const REG_LEN: usize = size_of::<Word>();

/// 发送大量数据使用的标签，xB31DADA 是 BIGDATA 每个字符 % F 算出来的
pub const SEND_BULK_LABEL: usize = 0xB31DADA;
