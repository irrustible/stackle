#[cfg(target_arch="aarch64")]
mod aarch64;
#[cfg(target_arch="aarch64")]
pub use aarch64::*;

// not looking forward to this one: https://github.com/Amanieu/corosensei/blob/master/src/arch/arm.rs
// #[cfg(target_arch="arm")]
// mod arm;
// #[cfg(target_arch="arm")]
// pub use arm::*;

// #[cfg(target_arch="riscv32")]
// mod riscv32;
// #[cfg(target_arch="riscv32")]
// pub use riscv32::*;

#[cfg(target_arch="riscv64")]
mod riscv64;
#[cfg(target_arch="riscv64")]
pub use riscv64::*;

#[cfg(all(target_arch="x86", unix))]
mod x86_unix;
#[cfg(all(target_arch="x86", unix))]
pub use x86_unix::*;

#[cfg(all(target_arch="x86_64", unix))]
mod x86_64_unix;
#[cfg(all(target_arch="x86_64", unix))]
pub use x86_64_unix::*;

// #[cfg(all(target_arch="x86_64", windows))]
// mod x86_64_windows;
// #[cfg(all(target_arch="x86_64", windows))]
// pub use x86_64_windows::*;

#[cfg(all(
  not(target_arch="aarch64"),
  not(all(target_arch="x86_64", unix)),
  not(all(target_arch="x86",    unix)),
))]
compile_error!("Unsupported target platform!");
