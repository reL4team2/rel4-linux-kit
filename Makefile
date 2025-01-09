BUILD_DIR := build
TARGET := aarch64-sel4

# sel4 installation directory
SEL4_PREFIX :=  $(realpath .)/.env/seL4
loader_artifacts_dir := $(SEL4_PREFIX)/bin
loader := $(loader_artifacts_dir)/sel4-kernel-loader
loader_cli := $(loader_artifacts_dir)/sel4-kernel-loader-add-payload

app_crate := root-task
app := $(BUILD_DIR)/$(app_crate).elf

qemu_args := 
qemu_args += -drive file=mount.img,if=none,format=raw,id=x0
qemu_args += -device virtio-blk-device,drive=x0

# qemu_args += -netdev user,id=net0,hostfwd=tcp::6379-:6379
# qemu_args += -device virtio-net-device,netdev=net0
# qemu_args += -object filter-dump,id=net0,netdev=net0,file=packets.pcap

$(app): $(app).intermediate

CARGO_BUILD_ARGS := --target-dir $(abspath $(BUILD_DIR)/target) \
			--artifact-dir $(BUILD_DIR) \
			--target $(TARGET) \
			--release

# SEL4_TARGET_PREFIX is used by build.rs scripts of various rust-sel4 crates to locate seL4
# configuration and libsel4 headers.
.INTERMDIATE: $(app).intermediate
$(app).intermediate:
	cargo build $(CARGO_BUILD_ARGS) --workspace --exclude $(app_crate)
	cargo build $(CARGO_BUILD_ARGS) -p $(app_crate)

image := $(BUILD_DIR)/image.elf

# Append the payload to the loader using the loader CLI
$(image): $(app) $(loader) $(loader_cli)
	echo $(loader_cli) $(loader)
	$(loader_cli) \
		--loader $(loader) \
		--sel4-prefix $(SEL4_PREFIX) \
		--app $(app) \
		-o $@

qemu_cmd := \
	qemu-system-aarch64 \
		$(qemu_args) \
		-machine virt,virtualization=on -cpu cortex-a57 -m size=1G \
		-serial mon:stdio \
		-nographic \
		-kernel $(image)

run: $(image)
	$(qemu_cmd)
	rm $(image)

clean:
	rm -rf $(BUILD_DIR)

.PHONY: run clean
