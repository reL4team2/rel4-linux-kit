use common::ipc_trait;

#[ipc_trait]
pub trait UartIface {
    fn init(&self);
    fn putchar(&mut self, c: u8);
    fn getchar(&mut self, c: u8);
    fn puts(&mut self, bytes: &[u8]);
}
