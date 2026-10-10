mod portable;
mod types;
pub use portable::*;
pub use types::Vector;

#[cfg(all(test, target_arch = "x86_64"))]
mod tests;

#[cfg(test)]
mod conversion_tests;
