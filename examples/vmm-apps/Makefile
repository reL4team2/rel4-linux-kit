CROSS_COMPILE ?= aarch64-linux-musl-
CC := $(CROSS_COMPILE)gcc
SRCS := $(wildcard src/*.c)
DSTS := $(patsubst src/%.c,build/%.elf,$(SRCS))
ASMS := $(wildcard src/*.S)
DSTS += $(patsubst src/%.S,build/%.elf,$(ASMS))

all: $(DSTS)
	@echo $(DSTS)

build/%.elf: src/%.c
	@mkdir -p build
	$(CC) -static $< -o $@

build/%.elf: src/%.S
	@mkdir -p build
	$(CC) -nostdlib -T linker.lds $< -o $@

clean:
	@rm -f $(DSTS)

.PHONY: all clean
