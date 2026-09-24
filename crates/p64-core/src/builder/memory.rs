use std::alloc::{alloc_zeroed, Layout};

use crate::mempak::MEMPAK_SIZE;

// Create wrapper type

// Keep existing range implementations

mod types;
pub use types::{WriteCallback, Memory};
