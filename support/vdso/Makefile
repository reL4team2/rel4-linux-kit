CC := aarch64-linux-musl-gcc

all: vdso

vdso: src/vdso.c
	$(CC) -fPIC -shared -nostdlib -o vdso.so $< -T linker.ld
