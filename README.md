# RustVMM Hypervisor

A simple userspace hypervisor implemented in Rust, based on rust-vmm components.

## Usage

1. Ensure KVM is available on your system (Linux).
2. Build the project: `cargo build`
3. Run the hypervisor: `cargo run`

The hypervisor creates a minimal VM with 1MB memory, loads a simple program that sets rax to 42 and halts.

## Dependencies

- KVM (Linux kernel module)
- Rust 1.60+

## Troubleshooting

- Ensure you have KVM support: `lsmod | grep kvm`
- Run as root or with appropriate permissions.