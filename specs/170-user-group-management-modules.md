# Spec 170: User and Group Management Modules

## Feature Summary

Implement comprehensive user and group management modules equivalent to Ansible's `user` and `group` modules. These modules provide essential functionality for managing system accounts, security permissions, and access control in deployment automation scenarios. Cross-platform support ensures consistent user management across Linux, macOS, Windows, and BSD systems.

## Goals & Requirements

### Functional Requirements
- **user module**: Create, modify, delete, and manage user accounts with comprehensive options
- **group module**: Create, modify, delete, and manage groups with member management
- **Cross-platform support**: Handle platform-specific user/group systems (passwd/shadow, Active Directory, Directory Services)
- **Security compliance**: Proper password handling, secure defaults, audit trail
- **State management**: Idempotent operations with change detection

### Non-Functional Requirements
- Secure password handling with no plaintext storage in logs
- Platform-specific permission and security model compliance
- Comprehensive error handling for permission and system limitations
- Performance optimization for bulk operations
- Audit logging for security compliance

### Success Criteria
- Full user/group lifecycle management on all supported platforms
- Security audit compliance for password and permission handling
- Comprehensive test coverage including edge cases
- Performance benchmarks for user/group operations
- Integration with existing authentication systems

## API/Interface Design

### User Module Interface
```rust
pub struct UserArgs {
    pub name: String,                     // Required: username
    pub state: Option<UserState>,         // present, absent (default: present)
    pub uid: Option<u32>,                 // User ID
    pub gid: Option<u32>,                 // Primary group ID
    pub group: Option<String>,            // Primary group name
    pub groups: Option<Vec<String>>,      // Supplementary groups
    pub append: Option<bool>,             // Append to existing groups vs replace
    pub comment: Option<String>,          // User comment/GECOS field
    pub home: Option<String>,             // Home directory path
    pub create_home: Option<bool>,        // Create home directory
    pub move_home: Option<bool>,          // Move existing home directory
    pub skeleton: Option<String>,         // Skeleton directory for new homes
    pub shell: Option<String>,            // Login shell
    pub password: Option<String>,         // Encrypted password hash
    pub password_lock: Option<bool>,      // Lock/unlock user account
    pub expires: Option<i64>,             // Account expiration (Unix timestamp)
    pub password_expire_max: Option<u32>, // Password expiration days
    pub password_expire_min: Option<u32>, // Minimum password age
    pub generate_ssh_key: Option<bool>,   // Generate SSH key pair
    pub ssh_key_bits: Option<u32>,        // SSH key bits (default: 2048)
    pub ssh_key_type: Option<SshKeyType>, // SSH key type (rsa, ed25519, ecdsa)
    pub ssh_key_file: Option<String>,     // SSH key file path
    pub ssh_key_comment: Option<String>,  // SSH key comment
    pub ssh_key_passphrase: Option<String>, // SSH key passphrase
    pub system: Option<bool>,             // Create system user
    pub force: Option<bool>,              // Force operations (remove home, etc.)
    pub remove: Option<bool>,             // Remove user files when state=absent
    pub login_class: Option<String>,      // Login class (BSD systems)
    pub seuser: Option<String>,           // SELinux user (Linux)
    pub role: Option<String>,             // User role
    pub authorization: Option<String>,    // Authorization attribute
    pub profile: Option<String>,          // User profile
    pub non_unique: Option<bool>,         // Allow non-unique UID
    pub update_password: Option<UpdatePassword>, // When to update password
}

#[derive(Debug, Clone)]
pub enum UserState {
    Present,
    Absent,
}

#[derive(Debug, Clone)]
pub enum SshKeyType {
    Rsa,
    Ed25519,
    Ecdsa,
    Dsa,
}

#[derive(Debug, Clone)]
pub enum UpdatePassword {
    Always,
    OnCreate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub name: String,
    pub uid: u32,
    pub gid: u32,
    pub comment: String,
    pub home: String,
    pub shell: String,
    pub groups: Vec<String>,
    pub password_locked: bool,
    pub password_expires: Option<i64>,
    pub ssh_key_file: Option<String>,
    pub ssh_public_key: Option<String>,
}
```

### Group Module Interface
```rust
pub struct GroupArgs {
    pub name: String,                     // Required: group name
    pub state: Option<GroupState>,        // present, absent (default: present)
    pub gid: Option<u32>,                 // Group ID
    pub members: Option<Vec<String>>,     // Group members
    pub append: Option<bool>,             // Append vs replace members
    pub system: Option<bool>,             // Create system group
    pub non_unique: Option<bool>,         // Allow non-unique GID
    pub force: Option<bool>,              // Force operations
    pub local: Option<bool>,              // Local group vs domain group
}

#[derive(Debug, Clone)]
pub enum GroupState {
    Present,
    Absent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInfo {
    pub name: String,
    pub gid: u32,
    pub members: Vec<String>,
    pub system: bool,
}
```

## File and Package Structure

### Module Organization
```
src/modules/system/
├── mod.rs                     # System module declarations
├── user.rs                    # User management module
├── group.rs                   # Group management module
├── user_group_utils/
│   ├── mod.rs                 # User/group utilities
│   ├── password.rs            # Password handling and validation
│   ├── ssh_keys.rs            # SSH key generation and management
│   ├── validation.rs          # Username/group name validation
│   └── permissions.rs         # Permission and ownership utilities
├── platform/
│   ├── mod.rs                 # Platform-specific implementations
│   ├── unix/
│   │   ├── mod.rs            # Unix-like platform support
│   │   ├── linux.rs          # Linux-specific user/group operations
│   │   ├── macos.rs          # macOS Directory Services integration
│   │   ├── freebsd.rs        # FreeBSD user/group operations
│   │   └── passwd.rs         # Traditional passwd/shadow handling
│   ├── windows/
│   │   ├── mod.rs            # Windows platform support
│   │   ├── local_users.rs    # Local user/group management
│   │   └── active_directory.rs # Active Directory integration
│   └── common.rs             # Cross-platform abstractions
```

### Integration Points
- Update `src/modules/system/mod.rs` to include user/group modules
- Integrate with file operations for home directory management
- Connect with SSH key modules for key generation

## Implementation Details

### 1. Cross-Platform User Management
```rust
#[async_trait]
pub trait UserManager: Send + Sync {
    async fn get_user(&self, name: &str) -> Result<Option<UserInfo>, UserError>;
    async fn create_user(&self, args: &UserArgs) -> Result<UserInfo, UserError>;
    async fn modify_user(&self, args: &UserArgs) -> Result<UserInfo, UserError>;
    async fn delete_user(&self, name: &str, remove_files: bool) -> Result<(), UserError>;
    async fn set_password(&self, name: &str, password_hash: &str) -> Result<(), UserError>;
    async fn lock_user(&self, name: &str) -> Result<(), UserError>;
    async fn unlock_user(&self, name: &str) -> Result<(), UserError>;
    async fn generate_ssh_key(&self, args: &UserArgs) -> Result<SshKeyPair, UserError>;
}

#[cfg(unix)]
pub struct UnixUserManager {
    platform_specific: Box<dyn PlatformUserManager>,
}

#[cfg(unix)]
#[async_trait]
impl UserManager for UnixUserManager {
    async fn create_user(&self, args: &UserArgs) -> Result<UserInfo, UserError> {
        // Validate username
        validate_username(&args.name)?;
        
        // Check if user already exists
        if self.get_user(&args.name).await?.is_some() {
            if args.force.unwrap_or(false) {
                return self.modify_user(args).await;
            } else {
                return Err(UserError::UserAlreadyExists(args.name.clone()));
            }
        }
        
        // Build useradd command
        let mut cmd = Command::new("useradd");
        
        if let Some(uid) = args.uid {
            cmd.arg("-u").arg(uid.to_string());
            if args.non_unique.unwrap_or(false) {
                cmd.arg("-o");
            }
        }
        
        if let Some(gid) = args.gid {
            cmd.arg("-g").arg(gid.to_string());
        } else if let Some(group) = &args.group {
            cmd.arg("-g").arg(group);
        }
        
        if let Some(groups) = &args.groups {
            cmd.arg("-G").arg(groups.join(","));
        }
        
        if let Some(comment) = &args.comment {
            cmd.arg("-c").arg(comment);
        }
        
        if let Some(home) = &args.home {
            cmd.arg("-d").arg(home);
            if args.create_home.unwrap_or(true) {
                cmd.arg("-m");
            }
            if let Some(skeleton) = &args.skeleton {
                cmd.arg("-k").arg(skeleton);
            }
        } else if args.create_home.unwrap_or(true) {
            cmd.arg("-m");
        }
        
        if let Some(shell) = &args.shell {
            cmd.arg("-s").arg(shell);
        }
        
        if args.system.unwrap_or(false) {
            cmd.arg("-r");
        }
        
        cmd.arg(&args.name);
        
        // Execute command
        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(UserError::CommandFailed {
                command: "useradd".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        
        // Set password if provided
        if let Some(password) = &args.password {
            self.set_password(&args.name, password).await?;
        }
        
        // Generate SSH key if requested
        if args.generate_ssh_key.unwrap_or(false) {
            self.generate_ssh_key(args).await?;
        }
        
        // Return user info
        self.get_user(&args.name).await?
            .ok_or_else(|| UserError::UserNotFound(args.name.clone()))
    }
    
    async fn set_password(&self, name: &str, password_hash: &str) -> Result<(), UserError> {
        // Use chpasswd or usermod to set password
        let mut cmd = Command::new("usermod");
        cmd.arg("-p").arg(password_hash).arg(name);
        
        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(UserError::PasswordSetFailed {
                user: name.to_string(),
                error: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub struct MacOSUserManager;

#[cfg(target_os = "macos")]
#[async_trait]
impl PlatformUserManager for MacOSUserManager {
    async fn create_user_platform_specific(&self, args: &UserArgs) -> Result<(), UserError> {
        // Use dscl (Directory Service Command Line) for macOS
        let mut cmd = Command::new("dscl");
        cmd.arg(".").arg("create").arg(format!("/Users/{}", args.name));
        
        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(UserError::CommandFailed {
                command: "dscl".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        
        // Set additional attributes with dscl
        if let Some(uid) = args.uid {
            Command::new("dscl")
                .args(&[".", "create", &format!("/Users/{}", args.name), "UniqueID", &uid.to_string()])
                .output().await?;
        }
        
        if let Some(gid) = args.gid {
            Command::new("dscl")
                .args(&[".", "create", &format!("/Users/{}", args.name), "PrimaryGroupID", &gid.to_string()])
                .output().await?;
        }
        
        Ok(())
    }
}

#[cfg(windows)]
pub struct WindowsUserManager;

#[cfg(windows)]
#[async_trait]
impl UserManager for WindowsUserManager {
    async fn create_user(&self, args: &UserArgs) -> Result<UserInfo, UserError> {
        // Use PowerShell New-LocalUser cmdlet
        let mut ps_cmd = String::from("New-LocalUser");
        ps_cmd.push_str(&format!(" -Name '{}'", args.name));
        
        if let Some(description) = &args.comment {
            ps_cmd.push_str(&format!(" -Description '{}'", description));
        }
        
        if args.password_lock.unwrap_or(false) {
            ps_cmd.push_str(" -AccountNeverExpires -UserMayNotChangePassword");
        }
        
        let output = Command::new("powershell")
            .arg("-Command")
            .arg(&ps_cmd)
            .output()
            .await?;
            
        if !output.status.success() {
            return Err(UserError::CommandFailed {
                command: "powershell".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        
        self.get_user(&args.name).await?
            .ok_or_else(|| UserError::UserNotFound(args.name.clone()))
    }
}
```

### 2. Group Management Implementation
```rust
#[async_trait]
pub trait GroupManager: Send + Sync {
    async fn get_group(&self, name: &str) -> Result<Option<GroupInfo>, GroupError>;
    async fn create_group(&self, args: &GroupArgs) -> Result<GroupInfo, GroupError>;
    async fn modify_group(&self, args: &GroupArgs) -> Result<GroupInfo, GroupError>;
    async fn delete_group(&self, name: &str) -> Result<(), GroupError>;
    async fn add_member(&self, group: &str, user: &str) -> Result<(), GroupError>;
    async fn remove_member(&self, group: &str, user: &str) -> Result<(), GroupError>;
}

#[cfg(unix)]
#[async_trait]
impl GroupManager for UnixGroupManager {
    async fn create_group(&self, args: &GroupArgs) -> Result<GroupInfo, GroupError> {
        validate_groupname(&args.name)?;
        
        if self.get_group(&args.name).await?.is_some() {
            return self.modify_group(args).await;
        }
        
        let mut cmd = Command::new("groupadd");
        
        if let Some(gid) = args.gid {
            cmd.arg("-g").arg(gid.to_string());
            if args.non_unique.unwrap_or(false) {
                cmd.arg("-o");
            }
        }
        
        if args.system.unwrap_or(false) {
            cmd.arg("-r");
        }
        
        cmd.arg(&args.name);
        
        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(GroupError::CommandFailed {
                command: "groupadd".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        
        // Add members if specified
        if let Some(members) = &args.members {
            for member in members {
                self.add_member(&args.name, member).await?;
            }
        }
        
        self.get_group(&args.name).await?
            .ok_or_else(|| GroupError::GroupNotFound(args.name.clone()))
    }
    
    async fn add_member(&self, group: &str, user: &str) -> Result<(), GroupError> {
        let mut cmd = Command::new("usermod");
        cmd.arg("-a").arg("-G").arg(group).arg(user);
        
        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(GroupError::MemberAddFailed {
                group: group.to_string(),
                user: user.to_string(),
                error: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        
        Ok(())
    }
}
```

### 3. SSH Key Generation
```rust
use std::process::Command;

pub struct SshKeyGenerator;

impl SshKeyGenerator {
    pub async fn generate_key_pair(&self, args: &UserArgs) -> Result<SshKeyPair, UserError> {
        let key_type = args.ssh_key_type.as_ref().unwrap_or(&SshKeyType::Ed25519);
        let bits = args.ssh_key_bits.unwrap_or(match key_type {
            SshKeyType::Rsa => 4096,
            SshKeyType::Ed25519 => 256,
            SshKeyType::Ecdsa => 521,
            SshKeyType::Dsa => 2048,
        });
        
        let key_file = args.ssh_key_file.as_deref()
            .unwrap_or(&format!("{}/.ssh/id_{}", 
                self.get_user_home(&args.name)?, 
                key_type.to_string().to_lowercase()));
        
        let mut cmd = Command::new("ssh-keygen");
        cmd.arg("-t").arg(key_type.to_string().to_lowercase());
        cmd.arg("-b").arg(bits.to_string());
        cmd.arg("-f").arg(key_file);
        cmd.arg("-N").arg(args.ssh_key_passphrase.as_deref().unwrap_or(""));
        
        if let Some(comment) = &args.ssh_key_comment {
            cmd.arg("-C").arg(comment);
        } else {
            cmd.arg("-C").arg(format!("{}@{}", args.name, hostname::get()?));
        }
        
        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(UserError::SshKeyGenerationFailed {
                error: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        
        // Read generated keys
        let private_key = tokio::fs::read_to_string(key_file).await?;
        let public_key = tokio::fs::read_to_string(format!("{}.pub", key_file)).await?;
        
        // Set proper permissions
        set_file_permissions(Path::new(key_file), "600").await?;
        set_file_permissions(Path::new(&format!("{}.pub", key_file)), "644").await?;
        
        Ok(SshKeyPair {
            private_key_path: key_file.to_string(),
            public_key_path: format!("{}.pub", key_file),
            private_key,
            public_key: public_key.trim().to_string(),
            fingerprint: self.calculate_fingerprint(&public_key)?,
        })
    }
    
    fn calculate_fingerprint(&self, public_key: &str) -> Result<String, UserError> {
        use sha2::{Sha256, Digest};
        
        // Parse public key and calculate SHA256 fingerprint
        let key_data = public_key.split_whitespace().nth(1)
            .ok_or_else(|| UserError::InvalidSshKey)?;
        let decoded = base64::decode(key_data)?;
        let hash = Sha256::digest(&decoded);
        
        Ok(format!("SHA256:{}", base64::encode(&hash)))
    }
}

#[derive(Debug, Clone)]
pub struct SshKeyPair {
    pub private_key_path: String,
    pub public_key_path: String,
    pub private_key: String,
    pub public_key: String,
    pub fingerprint: String,
}
```

### 4. Password Security and Validation
```rust
use sha2::{Sha256, Sha512, Digest};
use rand::{thread_rng, Rng};

pub struct PasswordManager;

impl PasswordManager {
    pub fn hash_password(&self, password: &str, algorithm: HashAlgorithm) -> Result<String, PasswordError> {
        let salt = self.generate_salt();
        
        match algorithm {
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(format!("{}{}", salt, password));
                Ok(format!("$5${}${:x}", salt, hasher.finalize()))
            }
            HashAlgorithm::Sha512 => {
                let mut hasher = Sha512::new();
                hasher.update(format!("{}{}", salt, password));
                Ok(format!("$6${}${:x}", salt, hasher.finalize()))
            }
            HashAlgorithm::Bcrypt => {
                // Use bcrypt crate for bcrypt hashing
                unimplemented!("Bcrypt support requires additional dependency")
            }
        }
    }
    
    fn generate_salt(&self) -> String {
        let mut rng = thread_rng();
        let salt: Vec<u8> = (0..16).map(|_| rng.gen()).collect();
        base64::encode(&salt)
    }
    
    pub fn validate_password_strength(&self, password: &str) -> Result<(), PasswordError> {
        if password.len() < 8 {
            return Err(PasswordError::TooShort);
        }
        
        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_digit(10));
        let has_special = password.chars().any(|c| !c.is_alphanumeric());
        
        if !(has_upper && has_lower && has_digit && has_special) {
            return Err(PasswordError::TooWeak);
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum HashAlgorithm {
    Sha256,
    Sha512,
    Bcrypt,
}
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_user_creation() {
        if !running_as_root() {
            return; // Skip test if not running as root
        }
        
        let manager = get_platform_user_manager();
        let test_user = format!("testuser_{}", rand::random::<u32>());
        
        let args = UserArgs {
            name: test_user.clone(),
            comment: Some("Test user".to_string()),
            shell: Some("/bin/bash".to_string()),
            create_home: Some(true),
            ..Default::default()
        };
        
        let result = manager.create_user(&args).await.unwrap();
        assert_eq!(result.name, test_user);
        assert!(result.home.contains(&test_user));
        
        // Cleanup
        manager.delete_user(&test_user, true).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_group_creation() {
        if !running_as_root() {
            return;
        }
        
        let manager = get_platform_group_manager();
        let test_group = format!("testgroup_{}", rand::random::<u32>());
        
        let args = GroupArgs {
            name: test_group.clone(),
            ..Default::default()
        };
        
        let result = manager.create_group(&args).await.unwrap();
        assert_eq!(result.name, test_group);
        
        // Cleanup
        manager.delete_group(&test_group).await.unwrap();
    }
    
    #[test]
    fn test_password_hashing() {
        let password_manager = PasswordManager;
        let hash = password_manager.hash_password("testpassword", HashAlgorithm::Sha512).unwrap();
        
        assert!(hash.starts_with("$6$"));
        assert!(hash.len() > 50);
    }
    
    #[test]
    fn test_username_validation() {
        assert!(validate_username("validuser").is_ok());
        assert!(validate_username("user123").is_ok());
        assert!(validate_username("").is_err());
        assert!(validate_username("invalid user").is_err());
        assert!(validate_username("root").is_err()); // Reserved username
    }
}
```

### Integration Tests
```rust
// tests/modules/user_group_integration_tests.rs
#[tokio::test]
async fn test_user_group_workflow() {
    // Test complete workflow: create group, create user, add to group, modify, delete
}

#[tokio::test]
async fn test_ssh_key_generation() {
    // Test SSH key generation and permissions
}
```

### Security Tests
```rust
#[tokio::test]
async fn test_password_security() {
    // Test password handling never logs plaintext
    // Test secure defaults
    // Test permission handling
}
```

## Edge Cases & Error Handling

### Platform Limitations
- Handle different UID/GID ranges across platforms
- Manage platform-specific user attributes
- Deal with case sensitivity differences
- Handle permission model differences

### Security Considerations
- Never log plaintext passwords
- Secure temporary file handling
- Proper permission setting for SSH keys
- Validation of user/group names

### Error Recovery
```rust
#[derive(thiserror::Error, Debug)]
pub enum UserError {
    #[error("User already exists: {0}")]
    UserAlreadyExists(String),
    
    #[error("User not found: {0}")]
    UserNotFound(String),
    
    #[error("Invalid username: {name}")]
    InvalidUsername { name: String },
    
    #[error("Permission denied for user operation")]
    PermissionDenied,
    
    #[error("Command failed: {command}, error: {stderr}")]
    CommandFailed { command: String, stderr: String },
    
    #[error("Password operation failed for user {user}: {error}")]
    PasswordSetFailed { user: String, error: String },
    
    #[error("SSH key generation failed: {error}")]
    SshKeyGenerationFailed { error: String },
    
    #[error("Home directory operation failed: {error}")]
    HomeDirectoryError { error: String },
}

#[derive(thiserror::Error, Debug)]
pub enum GroupError {
    #[error("Group already exists: {0}")]
    GroupAlreadyExists(String),
    
    #[error("Group not found: {0}")]
    GroupNotFound(String),
    
    #[error("Invalid group name: {name}")]
    InvalidGroupName { name: String },
    
    #[error("Failed to add member {user} to group {group}: {error}")]
    MemberAddFailed { group: String, user: String, error: String },
    
    #[error("Command failed: {command}, error: {stderr}")]
    CommandFailed { command: String, stderr: String },
}
```

## Dependencies

### System Commands
- **Linux**: `useradd`, `usermod`, `userdel`, `groupadd`, `groupmod`, `groupdel`, `ssh-keygen`
- **macOS**: `dscl`, `ssh-keygen`, traditional Unix commands
- **Windows**: PowerShell cmdlets (`New-LocalUser`, `Set-LocalUser`, etc.)
- **FreeBSD/OpenBSD**: `pw` command, traditional Unix commands

### External Crates
- `nix = "0.30"` (already available for Unix) - Unix system calls
- `winapi = "0.3"` (already available for Windows) - Windows APIs
- `base64 = "0.22"` (already available) - SSH key encoding
- `sha2 = "0.10"` (already available) - Password hashing
- `rand = "0.8"` - Salt generation (new dependency)
- `hostname = "0.4"` (already available) - SSH key comments

### Internal Dependencies
- `crate::modules::files` - Home directory management
- `crate::modules::interface` - Module interface
- `crate::types::platform` - Platform detection

## Configuration

### Module Configuration
```rust
pub struct UserModuleConfig {
    pub default_shell: String,              // Default: /bin/bash
    pub default_home_base: String,          // Default: /home
    pub password_hash_algorithm: HashAlgorithm, // Default: Sha512
    pub ssh_key_default_type: SshKeyType,   // Default: Ed25519
    pub enforce_password_strength: bool,     // Default: true
    pub max_username_length: usize,         // Default: 32
    pub reserved_usernames: Vec<String>,    // Reserved names like root, daemon
}

pub struct GroupModuleConfig {
    pub max_groupname_length: usize,       // Default: 32
    pub reserved_groupnames: Vec<String>,  // Reserved names like root, wheel
}
```

### Environment Variables
- `RUSTLE_USER_DEFAULT_SHELL` - Default shell for new users
- `RUSTLE_USER_HOME_BASE` - Base directory for user homes
- `RUSTLE_USER_ENFORCE_PASSWORD_STRENGTH` - Enforce strong passwords

## Documentation

### Usage Examples
```yaml
# Basic user creation
- name: Create application user
  user:
    name: myapp
    comment: "Application User"
    shell: /bin/bash
    home: /opt/myapp
    create_home: yes
    system: yes

# User with SSH key
- name: Create deployment user with SSH key
  user:
    name: deploy
    groups: [sudo, docker]
    generate_ssh_key: yes
    ssh_key_type: ed25519
    ssh_key_comment: "deploy@{{ ansible_hostname }}"

# Group management
- name: Create application group
  group:
    name: myapp
    system: yes

- name: Add users to group
  group:
    name: developers
    members: [alice, bob, charlie]
    append: yes

# Password management
- name: Set user password
  user:
    name: alice
    password: "{{ 'mypassword' | password_hash('sha512') }}"
    update_password: on_create
```

This specification provides comprehensive user and group management capabilities essential for secure system administration and deployment automation.