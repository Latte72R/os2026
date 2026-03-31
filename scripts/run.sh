#!/usr/bin/env bash
set -eu

kernel="$1"

qemu-system-riscv64 \
  -machine virt \
  -bios default \
  -nographic \
  -serial mon:stdio \
  -kernel "$kernel"
