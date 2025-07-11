// Temporarily comment out conflicting modules to enable core compilation
// pub mod capabilities;
pub mod cache;
pub mod compiler;
// pub mod optimizer;
pub mod target_detection;
// pub mod toolchain;
// pub mod zero_infra;
// pub mod zigbuild;

// pub use capabilities::*;
pub use cache::*;
pub use compiler::*;
// pub use optimizer::*;
pub use target_detection::*;
// pub use toolchain::*;
// pub use zero_infra::*;
// pub use zigbuild::*;
