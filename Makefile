BUILD_DIR := target
TARGET := aarch64-sel4
QEMU_LOG ?= n

# sel4 installation directory
SEL4_PREFIX :=  $(realpath .)/.env/seL4
loader_artifacts_dir := $(SEL4_PREFIX)/bin
loader := $(loader_artifacts_dir)/sel4-kernel-loader
loader_cli := $(loader_artifacts_dir)/sel4-kernel-loader-add-payload

app_crate := root-task
app := $(BUILD_DIR)/$(app_crate).elf

qemu_args := -drive file=mount.img,if=none,format=raw,id=x0
qemu_args += -device virtio-blk-device,drive=x0

# qemu_args += -netdev user,id=net0,hostfwd=tcp::6379-:6379
# qemu_args += -device virtio-net-device,netdev=net0
# qemu_args += -object filter-dump,id=net0,netdev=net0,file=packets.pcap

ifeq ($(QEMU_LOG), y)
	qemu_args += -D qemu.log -d in_asm,int,pcall,cpu_reset,guest_errors
endif

CARGO_BUILD_ARGS := --artifact-dir $(BUILD_DIR) \
	--target $(TARGET) \
	--release

build: 
	cargo build $(CARGO_BUILD_ARGS) --workspace --exclude $(app_crate)
	cargo build $(CARGO_BUILD_ARGS) -p $(app_crate)

image := $(BUILD_DIR)/image.elf

# Append the payload to the loader using the loader CLI
buld_img: build $(loader) $(loader_cli)
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

run: buld_img
	$(qemu_cmd)
	@rm $(image)

busybox:
	wget https://github.com/Azure-stars/rust-root-task-demo-mi-dev/raw/refs/heads/main/busybox

clean:
	rm -rf $(BUILD_DIR)

test-examples: 
	@make -C examples/linux-apps/helloworld
	@./tools/ins_modify.py examples/linux-apps/helloworld/main.elf .env/example
	@cargo build $(CARGO_BUILD_ARGS) -p kernel-thread --features "example"
	@cargo build $(CARGO_BUILD_ARGS) --workspace --exclude $(app_crate) --exclude kernel-thread
	@cargo build $(CARGO_BUILD_ARGS) -p $(app_crate)
	@$(loader_cli) \
		--loader $(loader) \
		--sel4-prefix $(SEL4_PREFIX) \
		--app $(app) \
		-o $(image)
	$(qemu_cmd)
	@rm $(image)
	@make -C examples/linux-apps/sigtest
	@./tools/ins_modify.py examples/linux-apps/sigtest/main.elf .env/example
	@cargo build $(CARGO_BUILD_ARGS) -p kernel-thread --features "example"
	@cargo build $(CARGO_BUILD_ARGS) -p $(app_crate)
	@$(loader_cli) \
		--loader $(loader) \
		--sel4-prefix $(SEL4_PREFIX) \
		--app $(app) \
		-o $(image)
	$(qemu_cmd)
	@rm $(image)

.PHONY: run clean
