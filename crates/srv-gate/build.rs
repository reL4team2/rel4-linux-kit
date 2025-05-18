fn main() {
    println!(r#"cargo::rustc-check-cfg=cfg(ipc, values("uart", "blk"))"#);
}
