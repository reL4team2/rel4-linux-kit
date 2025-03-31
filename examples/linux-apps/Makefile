CROSS_COMPILE ?= aarch64-linux-musl-
CC := $(CROSS_COMPILE)gcc
SRCS := $(wildcard src/*.c)
DSTS := $(patsubst src/%.c,build/%,$(SRCS))

all: $(DSTS)
	@echo $(DSTS)

build/%: src/%.c
	@mkdir -p build
	$(CC) -static $< -o $@

clean:
	@rm -f $(DSTS)

.PHONY: all clean
