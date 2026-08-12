PROJECT := vertos
TARGET := riscv64imac-unknown-none-elf
PROFILE ?= release

KERNEL := target/$(TARGET)/$(PROFILE)/$(PROJECT)
KERNEL_BIN := $(KERNEL).bin
LINKER_SCRIPT := linker/kernel.ld
RUSTFLAGS := -C link-arg=-T$(LINKER_SCRIPT)

USER_ELF := target/$(TARGET)/release/shell
USER_BIN := target/$(TARGET)/release/shell.bin
USER_LINKER_SCRIPT := user/linker/user.ld
USER_RUSTFLAGS := -C link-arg=-T$(USER_LINKER_SCRIPT) -C relocation-model=static
OBJCOPY ?= llvm-objcopy

KERNEL_SOURCES := Makefile Cargo.toml Cargo.lock rust-toolchain.toml $(LINKER_SCRIPT) \
	$(wildcard src/*.rs src/arch/*.rs src/memory/*.rs src/platform/*.rs boot/*.S)
USER_SOURCES := Makefile user/Cargo.toml user/Cargo.lock $(USER_LINKER_SCRIPT) \
	$(wildcard user/src/*.rs user/boot/*.S)

.DEFAULT_GOAL := help

.PHONY: help fmt check build user run test test-shell clean

help: ## Show help
	@awk 'BEGIN {FS = ":.*?## "}; /^[a-zA-Z_-]+:.*?## / {printf "\033[36m%-10s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

fmt: ## Format Rust sources
	cargo fmt --all

check: $(USER_BIN) ## Check the kernel without building
	RUSTFLAGS="$(RUSTFLAGS)" cargo check --target $(TARGET)

$(USER_ELF): $(USER_SOURCES)
	RUSTFLAGS="$(USER_RUSTFLAGS)" cargo build -p shell --target $(TARGET) --release

$(USER_BIN): $(USER_ELF)
	$(OBJCOPY) -O binary $< $@

user: $(USER_BIN) ## Build the embedded U-mode program

$(KERNEL): $(KERNEL_SOURCES) $(USER_BIN)
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --target $(TARGET) --$(PROFILE)

$(KERNEL_BIN): $(KERNEL)
	$(OBJCOPY) -O binary $< $@

build: $(KERNEL_BIN) ## Build the kernel and its raw binary

run: build ## Run the kernel with QEMU
	./scripts/run.sh $(KERNEL)

test: $(USER_BIN) ## Run kernel tests with QEMU
	RUSTFLAGS="$(RUSTFLAGS)" \
	CARGO_TARGET_RISCV64IMAC_UNKNOWN_NONE_ELF_RUNNER=./scripts/run.sh \
	cargo test --target $(TARGET) --lib

test-shell: build ## Exercise the U-mode shell with scripted QEMU input
	python3 scripts/test_shell.py $(KERNEL)

clean: ## Remove build artifacts
	cargo clean
