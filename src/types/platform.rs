#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    FreeBSD,
    Unknown(String),
}
