//! Test data builders for creating module arguments

use rustle_deploy::modules::files::FileState;
use rustle_deploy::modules::interface::{ModuleArgs, SpecialParameters};
use serde_json::Value;
use std::collections::HashMap;

/// Builder for file module test arguments
pub struct FileTestBuilder {
    path: Option<String>,
    state: Option<FileState>,
    mode: Option<String>,
    owner: Option<String>,
    group: Option<String>,
    src: Option<String>,
    backup: Option<bool>,
    force: Option<bool>,
    follow: Option<bool>,
    recurse: Option<bool>,
}

impl FileTestBuilder {
    pub fn new() -> Self {
        Self {
            path: None,
            state: None,
            mode: None,
            owner: None,
            group: None,
            src: None,
            backup: None,
            force: None,
            follow: None,
            recurse: None,
        }
    }

    pub fn path<S: Into<String>>(mut self, path: S) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn state(mut self, state: FileState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn mode<S: Into<String>>(mut self, mode: S) -> Self {
        self.mode = Some(mode.into());
        self
    }

    pub fn owner<S: Into<String>>(mut self, owner: S) -> Self {
        self.owner = Some(owner.into());
        self
    }

    pub fn group<S: Into<String>>(mut self, group: S) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn src<S: Into<String>>(mut self, src: S) -> Self {
        self.src = Some(src.into());
        self
    }

    pub fn backup(mut self, backup: bool) -> Self {
        self.backup = Some(backup);
        self
    }

    pub fn force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }

    pub fn follow(mut self, follow: bool) -> Self {
        self.follow = Some(follow);
        self
    }

    pub fn recurse(mut self, recurse: bool) -> Self {
        self.recurse = Some(recurse);
        self
    }

    pub fn build(self) -> ModuleArgs {
        let mut args = HashMap::new();

        if let Some(path) = self.path {
            args.insert("path".to_string(), Value::String(path));
        }

        if let Some(state) = self.state {
            let state_str = match state {
                FileState::Present => "present",
                FileState::Absent => "absent",
                FileState::Directory => "directory",
                FileState::Link => "link",
                FileState::Hard => "hard",
                FileState::Touch => "touch",
            };
            args.insert("state".to_string(), Value::String(state_str.to_string()));
        }

        if let Some(mode) = self.mode {
            args.insert("mode".to_string(), Value::String(mode));
        }

        if let Some(owner) = self.owner {
            args.insert("owner".to_string(), Value::String(owner));
        }

        if let Some(group) = self.group {
            args.insert("group".to_string(), Value::String(group));
        }

        if let Some(src) = self.src {
            args.insert("src".to_string(), Value::String(src));
        }

        if let Some(backup) = self.backup {
            args.insert("backup".to_string(), Value::Bool(backup));
        }

        if let Some(force) = self.force {
            args.insert("force".to_string(), Value::Bool(force));
        }

        if let Some(follow) = self.follow {
            args.insert("follow".to_string(), Value::Bool(follow));
        }

        if let Some(recurse) = self.recurse {
            args.insert("recurse".to_string(), Value::Bool(recurse));
        }

        ModuleArgs {
            args,
            special: SpecialParameters::default(),
        }
    }
}

/// Builder for copy module test arguments
pub struct CopyTestBuilder {
    src: Option<String>,
    dest: Option<String>,
    mode: Option<String>,
    owner: Option<String>,
    group: Option<String>,
    backup: Option<bool>,
    force: Option<bool>,
    follow: Option<bool>,
    preserve: Option<bool>,
    validate: Option<String>,
}

impl CopyTestBuilder {
    pub fn new() -> Self {
        Self {
            src: None,
            dest: None,
            mode: None,
            owner: None,
            group: None,
            backup: None,
            force: None,
            follow: None,
            preserve: None,
            validate: None,
        }
    }

    pub fn src<S: Into<String>>(mut self, src: S) -> Self {
        self.src = Some(src.into());
        self
    }

    pub fn dest<S: Into<String>>(mut self, dest: S) -> Self {
        self.dest = Some(dest.into());
        self
    }

    pub fn mode<S: Into<String>>(mut self, mode: S) -> Self {
        self.mode = Some(mode.into());
        self
    }

    pub fn owner<S: Into<String>>(mut self, owner: S) -> Self {
        self.owner = Some(owner.into());
        self
    }

    pub fn group<S: Into<String>>(mut self, group: S) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn backup(mut self, backup: bool) -> Self {
        self.backup = Some(backup);
        self
    }

    pub fn force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }

    pub fn follow(mut self, follow: bool) -> Self {
        self.follow = Some(follow);
        self
    }

    pub fn preserve(mut self, preserve: bool) -> Self {
        self.preserve = Some(preserve);
        self
    }

    pub fn validate<S: Into<String>>(mut self, validate: S) -> Self {
        self.validate = Some(validate.into());
        self
    }

    pub fn build(self) -> ModuleArgs {
        let mut args = HashMap::new();

        if let Some(src) = self.src {
            args.insert("src".to_string(), Value::String(src));
        }

        if let Some(dest) = self.dest {
            args.insert("dest".to_string(), Value::String(dest));
        }

        if let Some(mode) = self.mode {
            args.insert("mode".to_string(), Value::String(mode));
        }

        if let Some(owner) = self.owner {
            args.insert("owner".to_string(), Value::String(owner));
        }

        if let Some(group) = self.group {
            args.insert("group".to_string(), Value::String(group));
        }

        if let Some(backup) = self.backup {
            args.insert("backup".to_string(), Value::Bool(backup));
        }

        if let Some(force) = self.force {
            args.insert("force".to_string(), Value::Bool(force));
        }

        if let Some(follow) = self.follow {
            args.insert("follow".to_string(), Value::Bool(follow));
        }

        if let Some(preserve) = self.preserve {
            args.insert("preserve".to_string(), Value::Bool(preserve));
        }

        if let Some(validate) = self.validate {
            args.insert("validate".to_string(), Value::String(validate));
        }

        ModuleArgs {
            args,
            special: SpecialParameters::default(),
        }
    }
}

/// Builder for stat module test arguments
pub struct StatTestBuilder {
    path: Option<String>,
    get_checksum: Option<bool>,
    checksum_algorithm: Option<String>,
    get_attributes: Option<bool>,
    get_mime: Option<bool>,
    follow: Option<bool>,
}

impl StatTestBuilder {
    pub fn new() -> Self {
        Self {
            path: None,
            get_checksum: None,
            checksum_algorithm: None,
            get_attributes: None,
            get_mime: None,
            follow: None,
        }
    }

    pub fn path<S: Into<String>>(mut self, path: S) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn get_checksum(mut self, get_checksum: bool) -> Self {
        self.get_checksum = Some(get_checksum);
        self
    }

    pub fn checksum_algorithm<S: Into<String>>(mut self, algorithm: S) -> Self {
        self.checksum_algorithm = Some(algorithm.into());
        self
    }

    pub fn get_attributes(mut self, get_attributes: bool) -> Self {
        self.get_attributes = Some(get_attributes);
        self
    }

    pub fn get_mime(mut self, get_mime: bool) -> Self {
        self.get_mime = Some(get_mime);
        self
    }

    pub fn follow(mut self, follow: bool) -> Self {
        self.follow = Some(follow);
        self
    }

    pub fn build(self) -> ModuleArgs {
        let mut args = HashMap::new();

        if let Some(path) = self.path {
            args.insert("path".to_string(), Value::String(path));
        }

        if let Some(get_checksum) = self.get_checksum {
            args.insert("get_checksum".to_string(), Value::Bool(get_checksum));
        }

        if let Some(checksum_algorithm) = self.checksum_algorithm {
            args.insert(
                "checksum_algorithm".to_string(),
                Value::String(checksum_algorithm),
            );
        }

        if let Some(get_attributes) = self.get_attributes {
            args.insert("get_attributes".to_string(), Value::Bool(get_attributes));
        }

        if let Some(get_mime) = self.get_mime {
            args.insert("get_mime".to_string(), Value::Bool(get_mime));
        }

        if let Some(follow) = self.follow {
            args.insert("follow".to_string(), Value::Bool(follow));
        }

        ModuleArgs {
            args,
            special: SpecialParameters::default(),
        }
    }
}

/// Builder for template module test arguments
pub struct TemplateTestBuilder {
    src: Option<String>,
    dest: Option<String>,
    mode: Option<String>,
    owner: Option<String>,
    group: Option<String>,
    backup: Option<bool>,
    force: Option<bool>,
    variables: Option<HashMap<String, Value>>,
    trim_blocks: Option<bool>,
    lstrip_blocks: Option<bool>,
}

impl TemplateTestBuilder {
    pub fn new() -> Self {
        Self {
            src: None,
            dest: None,
            mode: None,
            owner: None,
            group: None,
            backup: None,
            force: None,
            variables: None,
            trim_blocks: None,
            lstrip_blocks: None,
        }
    }

    pub fn src<S: Into<String>>(mut self, src: S) -> Self {
        self.src = Some(src.into());
        self
    }

    pub fn dest<S: Into<String>>(mut self, dest: S) -> Self {
        self.dest = Some(dest.into());
        self
    }

    pub fn mode<S: Into<String>>(mut self, mode: S) -> Self {
        self.mode = Some(mode.into());
        self
    }

    pub fn owner<S: Into<String>>(mut self, owner: S) -> Self {
        self.owner = Some(owner.into());
        self
    }

    pub fn group<S: Into<String>>(mut self, group: S) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn backup(mut self, backup: bool) -> Self {
        self.backup = Some(backup);
        self
    }

    pub fn force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }

    pub fn variables(mut self, variables: HashMap<String, Value>) -> Self {
        self.variables = Some(variables);
        self
    }

    pub fn variable<K: Into<String>, V: Into<Value>>(mut self, key: K, value: V) -> Self {
        if self.variables.is_none() {
            self.variables = Some(HashMap::new());
        }
        self.variables
            .as_mut()
            .unwrap()
            .insert(key.into(), value.into());
        self
    }

    pub fn trim_blocks(mut self, trim_blocks: bool) -> Self {
        self.trim_blocks = Some(trim_blocks);
        self
    }

    pub fn lstrip_blocks(mut self, lstrip_blocks: bool) -> Self {
        self.lstrip_blocks = Some(lstrip_blocks);
        self
    }

    pub fn build(self) -> ModuleArgs {
        let mut args = HashMap::new();

        if let Some(src) = self.src {
            args.insert("src".to_string(), Value::String(src));
        }

        if let Some(dest) = self.dest {
            args.insert("dest".to_string(), Value::String(dest));
        }

        if let Some(mode) = self.mode {
            args.insert("mode".to_string(), Value::String(mode));
        }

        if let Some(owner) = self.owner {
            args.insert("owner".to_string(), Value::String(owner));
        }

        if let Some(group) = self.group {
            args.insert("group".to_string(), Value::String(group));
        }

        if let Some(backup) = self.backup {
            args.insert("backup".to_string(), Value::Bool(backup));
        }

        if let Some(force) = self.force {
            args.insert("force".to_string(), Value::Bool(force));
        }

        if let Some(variables) = self.variables {
            args.insert(
                "variables".to_string(),
                Value::Object(variables.into_iter().collect()),
            );
        }

        if let Some(trim_blocks) = self.trim_blocks {
            args.insert("trim_blocks".to_string(), Value::Bool(trim_blocks));
        }

        if let Some(lstrip_blocks) = self.lstrip_blocks {
            args.insert("lstrip_blocks".to_string(), Value::Bool(lstrip_blocks));
        }

        ModuleArgs {
            args,
            special: SpecialParameters::default(),
        }
    }
}

impl Default for FileTestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for CopyTestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for StatTestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TemplateTestBuilder {
    fn default() -> Self {
        Self::new()
    }
}
