#!/usr/bin/env bash
set -euo pipefail

export RUSTVMM_KERNEL=/home/tonywei/hhy/util/specVMM/linux/arch/x86/boot/bzImage
export RUSTVMM_DISK=/home/tonywei/hhy/util/specVMM/disk.raw
export RUSTVMM_SEED=/home/tonywei/hhy/util/specVMM/seed.iso

cargo test -- --ignored
