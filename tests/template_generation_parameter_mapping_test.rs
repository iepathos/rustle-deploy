use rustle_deploy::template::{BinaryTemplateGenerator, TargetInfo, TemplateConfig};
use rustle_deploy::types::platform::Platform;

#[tokio::test]
async fn test_template_generation_includes_parameter_handlers() {
    let config = TemplateConfig::default();
    let generator = BinaryTemplateGenerator::new(config).unwrap();

    // Create a minimal target info for testing
    let target_info = TargetInfo {
        target_triple: "x86_64-unknown-linux-gnu".to_string(),
        platform: Platform::Linux,
        architecture: "x86_64".to_string(),
        os_family: "unix".to_string(),
        libc: Some("gnu".to_string()),
        features: vec![],
    };

    // Create a simple module spec that would use parameter mapping
    let modules = vec![];

    // Generate module implementations
    let implementations = generator
        .generate_module_implementations(&modules, &target_info.platform)
        .unwrap();

    // Check that all essential parameter handlers are included
    let essential_handlers = [
        "modules/parameter_mapping/handlers/file.rs",
        "modules/parameter_mapping/handlers/copy.rs",
        "modules/parameter_mapping/handlers/command.rs",
        "modules/parameter_mapping/handlers/debug.rs",
        "modules/parameter_mapping/handlers/package.rs",
        "modules/parameter_mapping/handlers/service.rs",
        "modules/parameter_mapping/handlers/mod.rs",
        "modules/parameter_mapping/mapper.rs",
        "modules/parameter_mapping/error.rs",
    ];

    let mut missing_handlers = Vec::new();

    for handler_path in &essential_handlers {
        if !implementations.contains_key(*handler_path) {
            missing_handlers.push(*handler_path);
        }
    }

    assert!(
        missing_handlers.is_empty(),
        "Template generation is missing essential parameter handlers: {missing_handlers:?}"
    );

    // Verify that the handlers contain the expected content
    let file_handler = implementations
        .get("modules/parameter_mapping/handlers/file.rs")
        .unwrap();
    assert!(file_handler.contains("FileParameterHandler"));
    assert!(file_handler.contains("path"));
    assert!(file_handler.contains("state"));

    let copy_handler = implementations
        .get("modules/parameter_mapping/handlers/copy.rs")
        .unwrap();
    assert!(copy_handler.contains("CopyParameterHandler"));
    assert!(copy_handler.contains("src"));
    assert!(copy_handler.contains("dest"));

    // Verify the mod.rs exports all handlers
    let mod_file = implementations
        .get("modules/parameter_mapping/handlers/mod.rs")
        .unwrap();
    assert!(mod_file.contains("pub mod file"));
    assert!(mod_file.contains("pub mod copy"));
    assert!(mod_file.contains("pub use file::FileParameterHandler"));
    assert!(mod_file.contains("pub use copy::CopyParameterHandler"));

    // Verify the mapper includes all handlers
    let mapper_file = implementations
        .get("modules/parameter_mapping/mapper.rs")
        .unwrap();
    assert!(mapper_file.contains("FileParameterHandler"));
    assert!(mapper_file.contains("CopyParameterHandler"));
    assert!(mapper_file.contains(r#""file".to_string(), Box::new(FileParameterHandler)"#));
    assert!(mapper_file.contains(r#""copy".to_string(), Box::new(CopyParameterHandler)"#));
}

#[tokio::test]
async fn test_template_generation_parameter_mapping_consistency() {
    let config = TemplateConfig::default();
    let generator = BinaryTemplateGenerator::new(config).unwrap();

    let target_info = TargetInfo {
        target_triple: "x86_64-unknown-linux-gnu".to_string(),
        platform: Platform::Linux,
        architecture: "x86_64".to_string(),
        os_family: "unix".to_string(),
        libc: Some("gnu".to_string()),
        features: vec![],
    };

    let modules = vec![];
    let implementations = generator
        .generate_module_implementations(&modules, &target_info.platform)
        .unwrap();

    // Extract the mapper file content
    let mapper_content = implementations
        .get("modules/parameter_mapping/mapper.rs")
        .unwrap();

    // Extract the mod.rs content to see what handlers are exported
    let mod_content = implementations
        .get("modules/parameter_mapping/handlers/mod.rs")
        .unwrap();

    // Find all handlers mentioned in mod.rs
    let mod_handlers: Vec<&str> = mod_content
        .lines()
        .filter_map(|line| {
            if line.starts_with("pub mod ") {
                Some(line.trim_start_matches("pub mod ").trim_end_matches(";"))
            } else {
                None
            }
        })
        .collect();

    // Find all handlers registered in mapper.rs
    let mapper_registrations: Vec<&str> = mapper_content
        .lines()
        .filter_map(|line| {
            if line.contains("handlers.insert(") && line.contains(".to_string()") {
                // Extract module name from handlers.insert("module_name".to_string(), ...)
                let start = line.find('"')? + 1;
                let end = line.find("\".to_string()")?;
                Some(&line[start..end])
            } else {
                None
            }
        })
        .collect();

    println!("Handlers in mod.rs: {mod_handlers:?}");
    println!("Handlers registered in mapper.rs: {mapper_registrations:?}");

    // Essential handlers that should be both exported and registered
    let essential_handler_names = ["file", "copy", "command", "debug", "package", "service"];

    for handler_name in &essential_handler_names {
        assert!(
            mod_handlers.contains(handler_name),
            "Handler '{handler_name}' should be declared in mod.rs but wasn't found"
        );

        assert!(
            mapper_registrations.contains(handler_name),
            "Handler '{handler_name}' should be registered in mapper.rs but wasn't found"
        );
    }

    // Verify that shell is an alias for command
    assert!(
        mapper_registrations.contains(&"shell"),
        "Shell should be registered as an alias for command handler"
    );
}

#[test]
fn test_parameter_handler_file_structure() {
    // This test ensures that our parameter handlers are organized correctly
    // and can be found by the template generation system

    let file_handler_path =
        std::path::Path::new("src/templates/modules/parameter_mapping/handlers/file.rs");
    let copy_handler_path =
        std::path::Path::new("src/templates/modules/parameter_mapping/handlers/copy.rs");
    let mod_path = std::path::Path::new("src/templates/modules/parameter_mapping/handlers/mod.rs");
    let mapper_path = std::path::Path::new("src/templates/modules/parameter_mapping/mapper.rs");

    assert!(
        file_handler_path.exists(),
        "File handler should exist at expected path"
    );
    assert!(
        copy_handler_path.exists(),
        "Copy handler should exist at expected path"
    );
    assert!(mod_path.exists(), "Module declaration file should exist");
    assert!(mapper_path.exists(), "Parameter mapper should exist");

    // Read the files to ensure they have the expected content structure
    let file_content = std::fs::read_to_string(file_handler_path).unwrap();
    assert!(file_content.contains("pub struct FileParameterHandler"));
    assert!(file_content.contains("impl ModuleParameterHandler for FileParameterHandler"));

    let copy_content = std::fs::read_to_string(copy_handler_path).unwrap();
    assert!(copy_content.contains("pub struct CopyParameterHandler"));
    assert!(copy_content.contains("impl ModuleParameterHandler for CopyParameterHandler"));

    let mod_content = std::fs::read_to_string(mod_path).unwrap();
    assert!(mod_content.contains("pub mod file;"));
    assert!(mod_content.contains("pub mod copy;"));
    assert!(mod_content.contains("pub use file::FileParameterHandler;"));
    assert!(mod_content.contains("pub use copy::CopyParameterHandler;"));

    let mapper_content = std::fs::read_to_string(mapper_path).unwrap();
    assert!(mapper_content.contains("FileParameterHandler"));
    assert!(mapper_content.contains("CopyParameterHandler"));
}
