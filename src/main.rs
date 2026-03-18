use kvm_bindings::{kvm_segment, kvm_userspace_memory_region};
use kvm_ioctls::{Kvm, VcpuExit};
use linux_boot_params::boot_params::{boot_e820_entry, BootParams};
use linux_loader::loader::bzimage::BzImage;
use linux_loader::loader::{load_cmdline, Cmdline, KernelLoader};
use std::fs::File;
use std::io::{self, Write};
use vm_memory::{Bytes, GuestAddress, GuestMemoryBackend, GuestMemoryMmap};

const KERNEL_PATH: &str = "/home/tonywei/hhy/util/specVMM/linux/arch/x86/boot/bzImage";
const DISK_PATH: &str = "/home/tonywei/hhy/util/specVMM/disk.raw";
const SEED_PATH: &str = "/home/tonywei/hhy/util/specVMM/seed.iso";

const MEM_SIZE: u64 = 512 << 20; // 512 MiB
const KERNEL_LOAD_ADDR: u64 = 0x100000;
const BOOT_PARAMS_ADDR: u64 = 0x7000;
const CMDLINE_ADDR: u64 = 0x20000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1) Open /dev/kvm and create a VM.
    let kvm = Kvm::new()?;
    let vm = kvm.create_vm()?;

    // 2) Map guest memory.
    let guest_addr = GuestAddress(0);
    let mem: GuestMemoryMmap<()> =
        GuestMemoryMmap::from_ranges(&[(guest_addr, MEM_SIZE as usize)])?;
    let host_addr = mem.get_host_address(guest_addr)? as u64;

    let region = kvm_userspace_memory_region {
        slot: 0,
        guest_phys_addr: guest_addr.0,
        memory_size: MEM_SIZE,
        userspace_addr: host_addr,
        flags: 0,
    };
    unsafe { vm.set_user_memory_region(region)? };

    // 3) Load bzImage kernel.
    let mut kernel_file = File::open(KERNEL_PATH)?;
    let mut boot_params = BootParams::default();

    // Basic E820 memory map (1 RAM region covering all RAM).
    boot_params.e820_entries = 1;
    boot_params.e820_table[0] = boot_e820_entry {
        addr: 0,
        size: MEM_SIZE,
        type_: 1,
    };

    let kernel = BzImage::new();
    kernel.load(&mem, GuestAddress(KERNEL_LOAD_ADDR), &mut kernel_file, Some(&mut boot_params))?;

    // Kernel command line.
    let mut cmdline = Cmdline::new(4096)?;
    cmdline.insert_str("console=ttyS0 root=/dev/vda1 rw i8042.nokbd reboot=t panic=1")?;
    load_cmdline(&mem, GuestAddress(CMDLINE_ADDR), &cmdline)?;
    boot_params.hdr.cmd_line_ptr = CMDLINE_ADDR as u32;
    boot_params.hdr.cmdline_size = cmdline.as_str().len() as u32;

    // Write boot params (zero page).
    mem.write_obj(boot_params, GuestAddress(BOOT_PARAMS_ADDR))?;

    // 4) Create a vCPU and configure protected mode segments.
    let mut vcpu = vm.create_vcpu(0)?;
    let mut sregs = vcpu.get_sregs()?;

    // Flat segments for 32-bit protected mode.
    sregs.gdt.base = 0;
    sregs.gdt.limit = 0;

    sregs.cs = make_segment(0x8, 0xb);
    sregs.ds = make_segment(0x10, 0x3);
    sregs.es = make_segment(0x10, 0x3);
    sregs.fs = make_segment(0x10, 0x3);
    sregs.gs = make_segment(0x10, 0x3);
    sregs.ss = make_segment(0x10, 0x3);

    sregs.cr0 |= 0x1; // PE: protected mode
    vcpu.set_sregs(&sregs)?;

    let mut regs = vcpu.get_regs()?;
    regs.rip = KERNEL_LOAD_ADDR;
    regs.rsi = BOOT_PARAMS_ADDR;
    regs.rflags = 0x2;
    regs.rsp = 0x8000;
    vcpu.set_regs(&regs)?;

    // 5) Run loop and handle serial I/O + HLT.
    loop {
        match vcpu.run()? {
            VcpuExit::Hlt => {
                println!("VcpuExit::Hlt");
                break;
            }
            VcpuExit::IoOut(port, data) => {
                handle_io_out(port, data)?;
            }
            VcpuExit::IoIn(port, data) => {
                handle_io_in(port, data);
            }
            exit_reason => {
                return Err(format!("Unexpected vCPU exit: {:?}", exit_reason).into());
            }
        }
    }

    Ok(())
}

fn make_segment(selector: u16, type_: u8) -> kvm_segment {
    kvm_segment {
        base: 0,
        limit: 0xffff_ffff,
        selector,
        type_,
        present: 1,
        dpl: 0,
        db: 1,
        s: 1,
        l: 0,
        g: 1,
        avl: 0,
        unusable: 0,
        padding: 0,
    }
}

fn handle_io_out(port: u16, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    if port == 0x3f8 && data.len() == 1 {
        print!("{}", data[0] as char);
        io::stdout().flush()?;
        return Ok(());
    }

    if (0x3f8..=0x3ff).contains(&port) {
        return Ok(());
    }

    Err(format!("Unexpected IO out: port=0x{:x}", port).into())
}

fn handle_io_in(port: u16, data: &mut [u8]) {
    if data.is_empty() {
        return;
    }

    match port {
        // THR/RBR
        0x3f8 => data[0] = 0,
        // LSR: THR empty + TEMT
        0x3fd => data[0] = 0x60,
        _ if (0x3f8..=0x3ff).contains(&port) => data[0] = 0,
        _ => data[0] = 0,
    }
}
