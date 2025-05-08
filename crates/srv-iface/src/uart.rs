use common::ipc_trait;

#[ipc_trait]
pub trait UartIface: Sync + Send {
    fn init(&mut self);
    fn putchar(&mut self, c: u8);
    fn getchar(&mut self) -> u8;
    fn puts(&mut self, bytes: &[u8]);
}
