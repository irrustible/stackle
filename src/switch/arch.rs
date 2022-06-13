#[cfg(all(target_arch="x86_64", unix))]
mod x86_64_unix;
#[cfg(all(target_arch="x86_64", unix))]
pub use x86_64_unix::*;

// #[cfg(all(target_arch="x86_64", windows))]
// mod x86_64_windows;
// #[cfg(all(target_arch="x86_64", windows))]
// pub use x86_64_windows::*;
