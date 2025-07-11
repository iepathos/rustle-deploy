#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    FreeBSD,
    Unknown(String),
}
