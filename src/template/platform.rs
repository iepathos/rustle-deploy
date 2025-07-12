use crate::types::platform::Platform;
use anyhow::Result;
use thiserror::Error;

use super::{GeneratedTemplate, TargetInfo, TemplateError};

#[derive(Error, Debug)]
pub enum PlatformError {
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),
    #[error("Platform-specific generation failed: {0}")]
    GenerationFailed(String),
    #[error("Target configuration error: {0}")]
    TargetConfiguration(String),
}

/// Platform-specific template generation
pub trait PlatformTemplateGenerator {
    fn generate_platform_specific_code(
        &self,
        template: &mut GeneratedTemplate,
        target_info: &TargetInfo,
    ) -> Result<(), PlatformError>;

    fn add_platform_dependencies(&self, dependencies: &mut Vec<String>);
    fn get_compilation_flags(&self, target_info: &TargetInfo) -> Vec<String>;
    fn get_runtime_features(&self) -> Vec<String>;
}

/// Linux-specific template generation
pub struct LinuxTemplateGenerator;

impl PlatformTemplateGenerator for LinuxTemplateGenerator {
    fn generate_platform_specific_code(
        &self,
        template: &mut GeneratedTemplate,
        target_info: &TargetInfo,
    ) -> Result<(), PlatformError> {
        // Add Linux-specific code
        let linux_code = format!(
            r#"
#[cfg(target_os = "linux")]
mod platform {{
    use std::process::Command;
    use anyhow::Result;
    
    pub fn get_system_info() -> Result<SystemInfo> {{
        let output = Command::new("uname")
            .args(&["-a"])
            .output()?;
        
        let uname_output = String::from_utf8_lossy(&output.stdout);
        
        Ok(SystemInfo {{
            kernel_version: uname_output.lines().next().unwrap_or("unknown").to_string(),
            architecture: "{}".to_string(),
            libc: "{}".to_string(),
        }})
    }}
    
    pub fn setup_signal_handlers() -> Result<()> {{
        use nix::sys::signal::{{self, Signal}};
        use nix::unistd::Pid;
        
        extern "C" fn handle_sigterm(_: i32) {{
            std::process::exit(0);
        }}
        
        unsafe {{
            signal::signal(Signal::SIGTERM, signal::SigHandler::Handler(handle_sigterm))?;
            signal::signal(Signal::SIGINT, signal::SigHandler::Handler(handle_sigterm))?;
        }}
        
        Ok(())
    }}
    
    pub fn check_permissions() -> Result<bool> {{
        use nix::unistd::{{getuid, geteuid}};
        
        // Check if running as root or with appropriate permissions
        Ok(getuid().is_root() || geteuid().is_root())
    }}
    
    #[derive(Debug, Clone)]
    pub struct SystemInfo {{
        pub kernel_version: String,
        pub architecture: String,
        pub libc: String,
    }}
}}
"#,
            target_info.architecture,
            target_info.libc.as_deref().unwrap_or("unknown")
        );

        template.source_files.insert(
            std::path::PathBuf::from("src/platform/linux.rs"),
            linux_code,
        );

        Ok(())
    }

    fn add_platform_dependencies(&self, dependencies: &mut Vec<String>) {
        dependencies.push("nix = \"0.27\"".to_string());
        dependencies.push("libc = \"0.2\"".to_string());
    }

    fn get_compilation_flags(&self, target_info: &TargetInfo) -> Vec<String> {
        let mut flags = vec![];

        if target_info.libc.as_deref() == Some("musl") {
            flags.push("--target".to_string());
            flags.push(target_info.target_triple.clone());
        }

        flags
    }

    fn get_runtime_features(&self) -> Vec<String> {
        vec![
            "unix_socket".to_string(),
            "signal_handling".to_string(),
            "process_control".to_string(),
        ]
    }
}

/// macOS-specific template generation
pub struct MacOSTemplateGenerator;

impl PlatformTemplateGenerator for MacOSTemplateGenerator {
    fn generate_platform_specific_code(
        &self,
        template: &mut GeneratedTemplate,
        target_info: &TargetInfo,
    ) -> Result<(), PlatformError> {
        let macos_code = format!(
            r#"
#[cfg(target_os = "macos")]
mod platform {{
    use std::process::Command;
    use anyhow::Result;
    
    pub fn get_system_info() -> Result<SystemInfo> {{
        let output = Command::new("sw_vers")
            .output()?;
        
        let sw_vers_output = String::from_utf8_lossy(&output.stdout);
        
        Ok(SystemInfo {{
            version: sw_vers_output.lines()
                .find(|line| line.starts_with("ProductVersion:"))
                .map(|line| line.split_whitespace().nth(1).unwrap_or("unknown"))
                .unwrap_or("unknown")
                .to_string(),
            architecture: "{}".to_string(),
            build: sw_vers_output.lines()
                .find(|line| line.starts_with("BuildVersion:"))
                .map(|line| line.split_whitespace().nth(1).unwrap_or("unknown"))
                .unwrap_or("unknown")
                .to_string(),
        }})
    }}
    
    pub fn setup_signal_handlers() -> Result<()> {{
        use nix::sys::signal::{{self, Signal}};
        
        extern "C" fn handle_sigterm(_: i32) {{
            std::process::exit(0);
        }}
        
        unsafe {{
            signal::signal(Signal::SIGTERM, signal::SigHandler::Handler(handle_sigterm))?;
            signal::signal(Signal::SIGINT, signal::SigHandler::Handler(handle_sigterm))?;
        }}
        
        Ok(())
    }}
    
    pub fn check_sandbox_restrictions() -> Result<bool> {{
        // Check for macOS sandbox restrictions
        // This is a simplified check - real implementation would be more comprehensive
        std::fs::metadata("/System/Library/Sandbox").is_ok()
    }}
    
    #[derive(Debug, Clone)]
    pub struct SystemInfo {{
        pub version: String,
        pub architecture: String,
        pub build: String,
    }}
}}
"#,
            target_info.architecture
        );

        template.source_files.insert(
            std::path::PathBuf::from("src/platform/macos.rs"),
            macos_code,
        );

        Ok(())
    }

    fn add_platform_dependencies(&self, dependencies: &mut Vec<String>) {
        dependencies.push("nix = \"0.27\"".to_string());
        dependencies.push("core-foundation = \"0.9\"".to_string());
    }

    fn get_compilation_flags(&self, _target_info: &TargetInfo) -> Vec<String> {
        vec![
            "-C".to_string(),
            "link-arg=-framework".to_string(),
            "-C".to_string(),
            "link-arg=CoreFoundation".to_string(),
        ]
    }

    fn get_runtime_features(&self) -> Vec<String> {
        vec![
            "unix_socket".to_string(),
            "signal_handling".to_string(),
            "macos_permissions".to_string(),
        ]
    }
}

/// Windows-specific template generation
pub struct WindowsTemplateGenerator;

impl PlatformTemplateGenerator for WindowsTemplateGenerator {
    fn generate_platform_specific_code(
        &self,
        template: &mut GeneratedTemplate,
        target_info: &TargetInfo,
    ) -> Result<(), PlatformError> {
        let windows_code = format!(
            r#"
#[cfg(target_os = "windows")]
mod platform {{
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use winapi::um::sysinfoapi::GetSystemInfo;
    use winapi::um::winbase::GetComputerNameW;
    use anyhow::Result;
    
    pub fn get_system_info() -> Result<SystemInfo> {{
        unsafe {{
            let mut system_info = std::mem::zeroed();
            GetSystemInfo(&mut system_info);
            
            let mut computer_name = [0u16; 256];
            let mut size = computer_name.len() as u32;
            GetComputerNameW(computer_name.as_mut_ptr(), &mut size);
            
            let computer_name = OsString::from_wide(&computer_name[..size as usize])
                .into_string()
                .unwrap_or_else(|_| "unknown".to_string());
            
            Ok(SystemInfo {{
                computer_name,
                architecture: "{}".to_string(),
                processor_count: system_info.dwNumberOfProcessors,
            }})
        }}
    }}
    
    pub fn setup_signal_handlers() -> Result<()> {{
        use winapi::um::consoleapi::SetConsoleCtrlHandler;
        use winapi::um::wincon::{{CTRL_C_EVENT, CTRL_BREAK_EVENT}};
        use winapi::shared::minwindef::{{BOOL, DWORD, TRUE}};
        
        unsafe extern "system" fn ctrl_handler(ctrl_type: DWORD) -> BOOL {{
            match ctrl_type {{
                CTRL_C_EVENT | CTRL_BREAK_EVENT => {{
                    std::process::exit(0);
                }}
                _ => FALSE,
            }}
        }}
        
        unsafe {{
            SetConsoleCtrlHandler(Some(ctrl_handler), TRUE);
        }}
        
        Ok(())
    }}
    
    pub fn check_administrator_rights() -> Result<bool> {{
        use winapi::um::securitybaseapi::{{GetTokenInformation, TokenElevation}};
        use winapi::um::processthreadsapi::{{GetCurrentProcess, OpenProcessToken}};
        use winapi::um::handleapi::CloseHandle;
        use winapi::um::winnt::{{TOKEN_QUERY, TOKEN_ELEVATION, HANDLE}};
        use winapi::shared::minwindef::{{DWORD, FALSE}};
        
        unsafe {{
            let mut token: HANDLE = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == FALSE {{
                return Ok(false);
            }}
            
            let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
            let mut size: DWORD = std::mem::size_of::<TOKEN_ELEVATION>() as DWORD;
            
            let result = GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut _ as *mut _,
                size,
                &mut size,
            ) != FALSE && elevation.TokenIsElevated != 0;
            
            CloseHandle(token);
            Ok(result)
        }}
    }}
    
    #[derive(Debug, Clone)]
    pub struct SystemInfo {{
        pub computer_name: String,
        pub architecture: String,
        pub processor_count: u32,
    }}
}}
"#,
            target_info.architecture
        );

        template.source_files.insert(
            std::path::PathBuf::from("src/platform/windows.rs"),
            windows_code,
        );

        Ok(())
    }

    fn add_platform_dependencies(&self, dependencies: &mut Vec<String>) {
        dependencies.push("winapi = { version = \"0.3\", features = [\"winnt\", \"winsvc\", \"sysinfoapi\", \"consoleapi\", \"wincon\", \"securitybaseapi\", \"processthreadsapi\", \"handleapi\"] }".to_string());
    }

    fn get_compilation_flags(&self, _target_info: &TargetInfo) -> Vec<String> {
        vec!["-C".to_string(), "link-arg=/SUBSYSTEM:CONSOLE".to_string()]
    }

    fn get_runtime_features(&self) -> Vec<String> {
        vec![
            "windows_service".to_string(),
            "admin_rights".to_string(),
            "registry_access".to_string(),
        ]
    }
}

/// Platform template generator factory
pub struct PlatformTemplateGeneratorFactory;

impl PlatformTemplateGeneratorFactory {
    pub fn create(
        platform: &Platform,
    ) -> Result<Box<dyn PlatformTemplateGenerator>, PlatformError> {
        match platform {
            Platform::Linux => Ok(Box::new(LinuxTemplateGenerator)),
            Platform::MacOS => Ok(Box::new(MacOSTemplateGenerator)),
            Platform::Windows => Ok(Box::new(WindowsTemplateGenerator)),
            _ => Err(PlatformError::UnsupportedPlatform(format!("{platform:?}"))),
        }
    }

    pub fn get_supported_platforms() -> Vec<Platform> {
        vec![Platform::Linux, Platform::MacOS, Platform::Windows]
    }
}

/// Platform-specific template enhancements
pub fn enhance_template_for_platform(
    template: &mut GeneratedTemplate,
    target_info: &TargetInfo,
) -> Result<(), TemplateError> {
    let generator = PlatformTemplateGeneratorFactory::create(&target_info.platform)
        .map_err(|e| TemplateError::Generation(e.to_string()))?;

    // Generate platform-specific code
    generator
        .generate_platform_specific_code(template, target_info)
        .map_err(|e| TemplateError::Generation(e.to_string()))?;

    // Add platform-specific compilation flags
    let mut platform_flags = generator.get_compilation_flags(target_info);
    template.compilation_flags.append(&mut platform_flags);

    // Add platform-specific dependencies to Cargo.toml
    let mut platform_deps = Vec::new();
    generator.add_platform_dependencies(&mut platform_deps);

    if !platform_deps.is_empty() {
        let platform_section = format!(
            "\n[target.'{}'.dependencies]\n{}",
            target_info.target_triple,
            platform_deps.join("\n")
        );
        template.cargo_toml.push_str(&platform_section);
    }

    // Add platform module declaration to main.rs
    if let Some(main_rs) = template
        .source_files
        .get_mut(&std::path::PathBuf::from("src/main.rs"))
    {
        let platform_mod = match target_info.platform {
            Platform::Linux => "mod platform { pub use super::platform::linux::*; }",
            Platform::MacOS => "mod platform { pub use super::platform::macos::*; }",
            Platform::Windows => "mod platform { pub use super::platform::windows::*; }",
            _ => "",
        };

        // Insert after the use statements
        if let Some(pos) = main_rs.find("use anyhow::") {
            if let Some(end_pos) = main_rs[pos..].find('\n') {
                main_rs.insert_str(pos + end_pos + 1, &format!("\n{platform_mod}\n"));
            }
        }
    }

    Ok(())
}
