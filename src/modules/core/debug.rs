//! Debug module - displays messages and variables

use async_trait::async_trait;
use serde_json;
use std::collections::HashMap;

use crate::modules::{
    error::{ModuleExecutionError, ValidationError},
    interface::{
        ArgumentSpec, ExecutionContext, ExecutionModule, ModuleArgs, ModuleDocumentation,
        ModuleResult, Platform, ReturnValueSpec,
    },
};

/// Debug module - displays messages and variables
pub struct DebugModule;

#[async_trait]
impl ExecutionModule for DebugModule {
    fn name(&self) -> &'static str {
        "debug"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn supported_platforms(&self) -> &[Platform] {
        &[
            Platform::Linux,
            Platform::MacOS,
            Platform::Windows,
            Platform::FreeBSD,
            Platform::OpenBSD,
            Platform::NetBSD,
        ]
    }

    async fn execute(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        let msg = args
            .args
            .get("msg")
            .and_then(|v| v.as_str())
            .unwrap_or("Hello world!");

        let var_name = args.args.get("var").and_then(|v| v.as_str());

        let output = if let Some(var) = var_name {
            if let Some(value) = context.variables.get(var) {
                format!("{}: {}", var, serde_json::to_string_pretty(value)?)
            } else {
                format!("{var}: VARIABLE IS NOT DEFINED!")
            }
        } else {
            msg.to_string()
        };

        // Print to stdout for visibility
        println!("{}", output);

        Ok(ModuleResult {
            changed: false,
            failed: false,
            msg: Some(output),
            stdout: None,
            stderr: None,
            rc: Some(0),
            results: HashMap::new(),
            diff: None,
            warnings: Vec::new(),
            ansible_facts: HashMap::new(),
        })
    }

    fn validate_args(&self, _args: &ModuleArgs) -> Result<(), ValidationError> {
        // Debug module accepts any arguments
        Ok(())
    }

    async fn check_mode(
        &self,
        args: &ModuleArgs,
        context: &ExecutionContext,
    ) -> Result<ModuleResult, ModuleExecutionError> {
        // Debug module has no side effects, so check mode is same as execution
        self.execute(args, context).await
    }

    fn documentation(&self) -> ModuleDocumentation {
        ModuleDocumentation {
            description: "Print statements during execution".to_string(),
            arguments: vec![
                ArgumentSpec {
                    name: "msg".to_string(),
                    description: "The customized message that is printed. If omitted, prints a generic message.".to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: Some("Hello world!".to_string()),
                },
                ArgumentSpec {
                    name: "var".to_string(),
                    description: "A variable name to debug. Mutually exclusive with msg.".to_string(),
                    required: false,
                    argument_type: "str".to_string(),
                    default: None,
                },
            ],
            examples: vec![
                r#"debug:
    msg: "System {{ inventory_hostname }} has been configured successfully""#.to_string(),
                r#"debug:
    var: hostvars"#.to_string(),
            ],
            return_values: vec![
                ReturnValueSpec {
                    name: "msg".to_string(),
                    description: "The message that was printed".to_string(),
                    returned: "always".to_string(),
                    value_type: "str".to_string(),
                },
            ],
        }
    }
}
