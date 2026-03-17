use kvm_bindings::kvm_userspace_memory_region;
use kvm_ioctls::{Kvm, VcpuExit};
use vm_memory::{Bytes, GuestAddress, GuestMemoryMmap};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1) Open /dev/kvm and create a VM.
    let kvm = Kvm::new()?;
    let vm = kvm.create_vm()?;

    // 2) Map 4KB of anonymous guest memory.
    let mem_size: u64 = 0x1000;
    let guest_addr = GuestAddress(0);
    let mem = GuestMemoryMmap::from_ranges(&[(guest_addr, mem_size as usize)])?;
    let host_addr = mem.get_host_address(guest_addr)? as u64;

    let region = kvm_userspace_memory_region {
        slot: 0,
        guest_phys_addr: guest_addr.0,
        memory_size: mem_size,
        userspace_addr: host_addr,
        flags: 0,
    };
    unsafe { vm.set_user_memory_region(region)? };

    // 3) Create a vCPU and inject simple x86 real-mode code:
    //    mov ax, 0x42; hlt
    // Note: With only 4KB mapped, we stay in real mode. This sets AX (lower 16 bits of RAX).
    let vcpu = vm.create_vcpu(0)?;
    let code: [u8; 4] = [0xB8, 0x42, 0x00, 0xF4];
    mem.write(&code, guest_addr)?;

    let mut sregs = vcpu.get_sregs()?;
    sregs.cs.base = 0;
    sregs.cs.selector = 0;
    vcpu.set_sregs(&sregs)?;

    let mut regs = vcpu.get_regs()?;
    regs.rip = 0;
    regs.rflags = 0x2;
    regs.rsp = mem_size;
    vcpu.set_regs(&regs)?;

    // 4) Run loop and handle HLT exit.
    loop {
        match vcpu.run()? {
            VcpuExit::Hlt => {
                let regs = vcpu.get_regs()?;
                println!("VcpuExit::Hlt, RAX = 0x{:x}", regs.rax);
                break;
            }
            exit_reason => {
                return Err(format!("Unexpected vCPU exit: {:?}", exit_reason).into());
            }
        }
    }

    Ok(())
}



