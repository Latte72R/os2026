PROJECT := os2026
TARGET := riscv64imac-unknown-none-elf
PROFILE ?= release

KERNEL := target/$(TARGET)/$(PROFILE)/$(PROJECT)
LINKER_SCRIPT := linker/kernel.ld
RUSTFLAGS := -C link-arg=-T$(LINKER_SCRIPT)

.DEFAULT_GOAL := help

.PHONY: help fmt check build run test clean

help: ## Show help
	@awk 'BEGIN {FS = ":.*?## "}; /^[a-zA-Z_-]+:.*?## / {printf "\033[36m%-10s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

fmt: ## Format Rust sources
	cargo fmt --all

check: ## Check the kernel without building
	RUSTFLAGS="$(RUSTFLAGS)" cargo check --target $(TARGET)

build: $(KERNEL) ## Build the kernel

$(KERNEL): Cargo.toml Cargo.lock rust-toolchain.toml $(LINKER_SCRIPT) $(wildcard src/*.rs) $(wildcard boot/*.S)
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --target $(TARGET) --$(PROFILE)

run: $(KERNEL) ## Run the kernel with QEMU
	./scripts/run.sh $(KERNEL)

test: ## Run kernel tests with QEMU
	RUSTFLAGS="$(RUSTFLAGS)" \
	CARGO_TARGET_RISCV64IMAC_UNKNOWN_NONE_ELF_RUNNER=./scripts/run.sh \
	cargo test --target $(TARGET) --lib

clean: ## Remove build artifacts
	cargo clean
