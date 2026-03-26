use std::convert::TryFrom;
use std::env;
use std::path::PathBuf;

use vmm::{KernelConfig, VMMConfig, Vmm};

#[test]
fn kernel_config_accepts_quoted_or_unquoted_cmdline() {
    let quoted = r#"path=/tmp/kernel,cmdline="console=ttyS0 root=/dev/vda1""#;
    let unquoted = r#"path=/tmp/kernel,cmdline=console=ttyS0 root=/dev/vda1"#;

    let quoted_cfg = KernelConfig::try_from(quoted).expect("quoted cmdline should parse");
    let unquoted_cfg = KernelConfig::try_from(unquoted).expect("unquoted cmdline should parse");

    assert_eq!(quoted_cfg.cmdline.as_str(), unquoted_cfg.cmdline.as_str());
    assert_eq!(quoted_cfg.path, PathBuf::from("/tmp/kernel"));
}

#[test]
fn kernel_config_requires_path() {
    let missing_path = r#"cmdline="console=ttyS0""#;
    assert!(KernelConfig::try_from(missing_path).is_err());
}

#[test]
fn vmm_config_builder_rejects_zero_vcpu() {
    let res = VMMConfig::builder()
        .vcpu_config(Some("num=0"))
        .kernel_config(Some("path=/tmp/kernel"))
        .build();
    assert!(res.is_err());
}

/// Smoke test for host setup and basic VMM construction.
/// This is ignored by default because it needs a Linux host with /dev/kvm
/// and valid kernel/disk paths.
#[test]
#[ignore]
fn smoke_vmm_constructs_with_env_paths() {
    let kernel = env::var("RUSTVMM_KERNEL").expect("RUSTVMM_KERNEL must be set");
    let disk = env::var("RUSTVMM_DISK").expect("RUSTVMM_DISK must be set");
    let seed = env::var("RUSTVMM_SEED").ok();

    let kernel_cfg = format!(
        "path={},cmdline=console=ttyS0 root=/dev/vda1 rw rootwait rootfstype=ext4 init=/bin/bash panic=1",
        kernel
    );
    let block_cfg = format!("path={}", disk);

    let vmm_config = VMMConfig::builder()
        .memory_config(Some("size_mib=1024"))
        .vcpu_config(Some("num=1"))
        .kernel_config(Some(kernel_cfg.as_str()))
        .block_config(Some(block_cfg.as_str()))
        .build()
        .expect("Failed to build VMM config");

    let mut vmm = Vmm::try_from(vmm_config).expect("Failed to construct VMM");
    if let Some(seed_path) = seed {
        vmm.add_block_device_extra(PathBuf::from(seed_path), true)
            .expect("Failed to add seed device");
    }
}
