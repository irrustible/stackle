#[cfg(all(target_arch="aarch63", unix))]
mod aarch64_unix;
#[cfg(all(target_arch="aarch64", unix))]
pub use aarch64_unix::*;

// #[cfg(all(target_arch="arm", unix))]
// mod arm_unix;
// #[cfg(all(target_arch="arm", unix))]
// pub use arm_unix::*;

// #[cfg(all(target_arch="riscv32", unix))]
// mod riscv32_unix;
// #[cfg(all(target_arch="riscv32", unix))]
// pub use riscv32_unix::*;

// #[cfg(all(target_arch="riscv64", unix))]
// mod riscv64_unix;
// #[cfg(all(target_arch="riscv64", unix))]
// pub use riscv64_unix::*;

#[cfg(all(target_arch="x86", unix))]
mod x86_unix;
#[cfg(all(target_arch="x86", unix))]
pub use x86_unix::*;

// #[cfg(all(target_arch="x86", windows))]
// mod x86_windows;
// #[cfg(all(target_arch="x86", windows))]
// pub use x86_windows::*;

#[cfg(all(target_arch="x86_64", unix))]
mod x86_64_unix;
#[cfg(all(target_arch="x86_64", unix))]
pub use x86_64_unix::*;

// #[cfg(all(target_arch="x86_64", windows))]
// mod x86_64_windows;
// #[cfg(all(target_arch="x86_64", windows))]
// pub use x86_64_windows::*;
