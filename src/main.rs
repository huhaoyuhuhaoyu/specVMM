use std::convert::TryFrom;
use std::path::PathBuf;

use vmm::{Vmm, VMMConfig};

const KERNEL_PATH: &str = "/home/tonywei/hhy/util/specVMM/linux/arch/x86/boot/bzImage";
const DISK_PATH: &str = "/home/tonywei/hhy/util/specVMM/disk.raw";
const SEED_PATH: &str = "/home/tonywei/hhy/util/specVMM/seed.iso";

fn main() {
    let kernel_cfg = format!(
        "path={},cmdline=console=ttyS0 root=/dev/vda1 rw rootwait rootfstype=ext4 init=/bin/bash panic=1",
        KERNEL_PATH
    );
    let block_cfg = format!("path={}", DISK_PATH);

    let vmm_config = VMMConfig::builder()
        .memory_config(Some("size_mib=1024"))
        .vcpu_config(Some("num=1"))
        .kernel_config(Some(kernel_cfg.as_str()))
        .block_config(Some(block_cfg.as_str()))
        .build()
        .expect("Failed to build VMM config");

    let mut vmm = Vmm::try_from(vmm_config).expect("Failed to create VMM from config");
    vmm.add_block_device_extra(PathBuf::from(SEED_PATH), true)
        .expect("Failed to add seed device");
    vmm.run().expect("VMM run failed");
}

