fn main() {
    println!("cargo::rustc-check-cfg=cfg(uart_ipc)");
    println!("cargo::rustc-check-cfg=cfg(fs_ipc)");
}
