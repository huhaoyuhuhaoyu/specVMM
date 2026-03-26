#!/usr/bin/env bash
set -euo pipefail

# Simple performance probes for the VMM:
# 1) Build time (cold-ish)
# 2) Test binary build time
# 3) Run-time launch latency (time to VM start)
#
# Requires:
#   RUSTVMM_KERNEL, RUSTVMM_DISK, RUSTVMM_SEED

if [[ -z "${RUSTVMM_KERNEL:-}" || -z "${RUSTVMM_DISK:-}" ]]; then
  echo "Missing env vars. Set RUSTVMM_KERNEL and RUSTVMM_DISK (and optionally RUSTVMM_SEED)."
  exit 1
fi

if [[ ! -e "${RUSTVMM_KERNEL}" ]]; then
  echo "Kernel not found: ${RUSTVMM_KERNEL}"
  exit 1
fi

if [[ ! -e "${RUSTVMM_DISK}" ]]; then
  echo "Disk not found: ${RUSTVMM_DISK}"
  exit 1
fi

echo "=== Build (dev) ==="
/usr/bin/time -f "build_dev_sec=%e" cargo build

echo "=== Build (tests) ==="
/usr/bin/time -f "build_tests_sec=%e" cargo test --no-run

echo "=== Launch latency (VM start) ==="
# This runs the VMM. Stop it inside the guest with 'poweroff' when it finishes booting.
# If you need to force-stop, use kill from another terminal.
/usr/bin/time -f "vm_launch_sec=%e" cargo run
