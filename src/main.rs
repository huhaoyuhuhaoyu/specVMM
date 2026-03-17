use kvm_bindings::kvm_userspace_memory_region;
use kvm_ioctls::{Kvm, VcpuExit};
use vm_memory::{Bytes, GuestAddress, GuestMemoryBackend, GuestMemoryMmap};
use std::io::{self, Read, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1) Open /dev/kvm and create a VM.
    let kvm = Kvm::new()?;
    let vm = kvm.create_vm()?;

    // 2) Map 4KB of anonymous guest memory.
    let mem_size: u64 = 0x1000;
    let guest_addr = GuestAddress(0);
    let mem: GuestMemoryMmap<()> = GuestMemoryMmap::from_ranges(&[(guest_addr, mem_size as usize)])?;
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
    //    mov dx, 0x3f8; mov al, '>'; out dx, al; in al, dx; out dx, al; mov ax, 0x42; hlt
    // Note: With only 4KB mapped, we stay in real mode. This sets AX (lower 16 bits of RAX).
    let mut vcpu = vm.create_vcpu(0)?;
    let code: [u8; 12] = [
        0xBA, 0xF8, 0x03, // mov dx, 0x3f8
        0xB0, 0x3E,       // mov al, '>'
        0xEE,             // out dx, al
        0xEC,             // in al, dx
        0xEE,             // out dx, al
        0xB8, 0x42, 0x00, // mov ax, 0x42
        0xF4,             // hlt
    ];
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

    // 4) Run loop and handle exits (HLT and simple serial I/O).
    loop {
        match vcpu.run()? {
            VcpuExit::Hlt => {
                let regs = vcpu.get_regs()?;
                println!("VcpuExit::Hlt, RAX = 0x{:x}", regs.rax);
                break;
            }
            VcpuExit::IoOut(port, data) => {
                if port == 0x3f8 && data.len() == 1 {
                    print!("{}", data[0] as char);
                    io::stdout().flush()?;
                } else {
                    return Err(format!("Unexpected IO out: port=0x{:x}", port).into());
                }
            }
            VcpuExit::IoIn(port, data) => {
                if port == 0x3f8 && data.len() == 1 {
                    let mut buf = [0u8; 1];
                    io::stdin().read_exact(&mut buf)?;
                    data[0] = buf[0];
                } else {
                    return Err(format!("Unexpected IO in: port=0x{:x}", port).into());
                }
            }
            exit_reason => {
                return Err(format!("Unexpected vCPU exit: {:?}", exit_reason).into());
            }
        }
    }

    Ok(())
}
