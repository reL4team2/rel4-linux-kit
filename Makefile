# export RUSTFLAGS = --cfg=uart_ipc --cfg=blk_ipc
# export RUSTFLAGS = --cfg=uart_ipc
export RUSTFLAGS := --check-cfg=cfg(uart_ipc) --check-cfg=cfg(blk_ipc) --check-cfg=cfg(fs_ipc) 

include tools/autoconfig.mk

BUILD_DIR := target
TARGET := aarch64-sel4
QEMU_LOG ?= n

# sel4 installation directory
SEL4_PREFIX :=  $(realpath .)/.env/seL4
loader_artifacts_dir := $(SEL4_PREFIX)/bin
loader := $(loader_artifacts_dir)/sel4-kernel-loader
loader_cli := sel4-kernel-loader-add-payload

app_crate := root-task
app := $(BUILD_DIR)/$(app_crate)

qemu_args := -drive file=mount.img,if=none,format=raw,id=x0
qemu_args += -device virtio-blk-device,drive=x0
qemu_args += -device virtio-net-device,netdev=net0
qemu_args += -netdev user,id=net0,hostfwd=tcp::5555-:5555,hostfwd=udp::5555-:5555
# qemu_args += --trace "virtio_*" --trace "virtqueue_*"
# qemu_args += -netdev user,id=net0,hostfwd=tcp::6379-:6379
# qemu_args += -device virtio-net-device,netdev=net0
# qemu_args += -object filter-dump,id=net0,netdev=net0,file=packets.pcap

ifeq ($(QEMU_LOG), y)
	qemu_args += -D qemu.log -d in_asm,int,pcall,cpu_reset,guest_errors
endif
ifeq ($(DUMP_DTB), y)
	qemu_args += -machine dumpdtb=qemu.dtb
endif

CARGO_BUILD_ARGS := --artifact-dir $(BUILD_DIR) \
	--target $(TARGET) \
	--release

build: 
	cargo build $(CARGO_BUILD_ARGS) --workspace --exclude $(app_crate)
#	cargo build $(CARGO_BUILD_ARGS) -p uart-thread -p test-demo
	cargo build $(CARGO_BUILD_ARGS) -p $(app_crate)

image := $(BUILD_DIR)/image.elf

# Append the payload to the loader using the loader CLI
buld_img: build $(loader)
	$(loader_cli) \
		--loader $(loader) \
		--sel4-prefix $(SEL4_PREFIX) \
		--app $(app) \
		-o $(image)

qemu_cmd := \
	qemu-system-aarch64 \
		$(qemu_args) \
		-machine virt,virtualization=on -cpu cortex-a57 -m size=1G \
		-serial mon:stdio \
		-nographic \
		-kernel $(image)

testcases:
	wget -qO- https://github.com/reL4team2/rel4-linux-kit/releases/download/toolchain/testcases.tgz | tar -zxf - -C .

support/vdso/vdso.so: support/vdso/vdso.so
	make -C support/vdso

disk_img: testcases support/vdso/vdso.so
	mkdir -p mount
	dd if=/dev/zero of=mount.img bs=4M count=64
	sync
	# mkfs.ext4 -b 4096 mount.img
	# mkfs.vfat -F 32 mount.img
	mkfs.ext4 -b 4096 -F -O ^metadata_csum_seed mount.img
	sudo mount mount.img mount
	sudo cp -r testcases/* mount/
	sudo cp support/tests/init.sh mount/
	sudo cp support/vdso/vdso.so mount/
	sync
	sudo umount mount
	sync

run: buld_img
	$(qemu_cmd)
	@rm $(image)

examples:
	make -C examples/linux-apps all

busybox:
	wget https://github.com/Azure-stars/rust-root-task-demo-mi-dev/raw/refs/heads/main/busybox

clean:
	rm -rf $(BUILD_DIR)
	make -C examples/linux-apps clean

clippy:
	cargo clippy

cloc:
	cloc . --not-match-d=.env --not-match-d=target/

.PHONY: run clean examples
