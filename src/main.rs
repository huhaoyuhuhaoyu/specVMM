use std::io::Write;
use kvm_bindings::*;
use kvm_ioctls::{Kvm, VcpuExit};
use vm_memory::{GuestAddress, GuestMemoryMmap};

fn main() {
    // Create KVM instance
    let kvm = Kvm::new().expect("Failed to create KVM instance");

    // Create VM
    let vm = kvm.create_vm().expect("Failed to create VM");

    // Allocate guest memory
    let mem_size = 0x100000; // 1MB
    let guest_memory = GuestMemoryMmap::from_ranges(&[(GuestAddress(0), mem_size)])
        .expect("Failed to create guest memory");

    // Set up memory region
    let region = kvm_userspace_memory_region {
        slot: 0,
        guest_phys_addr: 0,
        memory_size: mem_size as u64,
        userspace_addr: guest_memory.get_host_address(GuestAddress(0)).unwrap() as u64,
        flags: 0,
    };
    vm.set_user_memory_region(region).expect("Failed to set memory region");

    // Write a simple program to guest memory: mov rax, 42; hlt
    let code = [
        0x48, 0xc7, 0xc0, 0x2a, 0x00, 0x00, 0x00, // mov rax, 42
        0xf4, // hlt
    ];
    guest_memory.write_slice(&code, GuestAddress(0)).expect("Failed to write code");

    // Create VCPU
    let vcpu = vm.create_vcpu(0).expect("Failed to create VCPU");

    // Set initial registers
    let mut regs = vcpu.get_regs().expect("Failed to get regs");
    regs.rip = 0;
    regs.rflags = 2; // IF flag set
    vcpu.set_regs(&regs).expect("Failed to set regs");

    // Run the VCPU
    loop {
        match vcpu.run().expect("Failed to run VCPU") {
            VcpuExit::Hlt => {
                println!("VCPU halted");
                break;
            }
            exit_reason => {
                println!("VCPU exited with reason: {:?}", exit_reason);
                break;
            }
        }
    }

    println!("Hypervisor finished");
}