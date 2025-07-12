//! Source control operations module

pub mod git;
pub mod utils;

pub use git::{GitArgs, GitModule, GitResult};
pub use utils::{CredentialError, CredentialHandler, SshError, SshManager};
