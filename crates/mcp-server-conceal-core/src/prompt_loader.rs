/*
   Manages prompt template loading with built-in fallback and user customization.
   Supports two built-in templates selectable via config:
   - "simple" (default): lightweight, works with 1.5B+ models
   - "detailed": comprehensive, requires 3B+ models
*/

use anyhow::Result;
use std::path::PathBuf;
use tracing::warn;
use crate::config::Config;

const BUILTIN_SIMPLE: &str = include_str!("templates/builtin_simple.md");
const BUILTIN_DETAILED: &str = include_str!("templates/builtin_detailed.md");

#[derive(Clone)]
pub struct PromptLoader {
    prompts_dir: PathBuf,
}

impl PromptLoader {
    pub fn new() -> Result<Self> {
        let project_dirs = Config::get_app_dirs()?;
        let data_dir = project_dirs.data_dir();
        let prompts_dir = data_dir.join("prompts");
        
        std::fs::create_dir_all(&prompts_dir)?;
        
        Ok(Self { prompts_dir })
    }
    
    pub fn load_prompt(&self, template_name: Option<&String>) -> Result<String> {
        match template_name.map(|s| s.as_str()) {
            None | Some("simple") => Ok(BUILTIN_SIMPLE.to_string()),
            Some("detailed") => Ok(BUILTIN_DETAILED.to_string()),
            Some(name) => {
                // Try loading from user prompts directory
                let prompt_path = self.prompts_dir.join(format!("{}.md", name));
                match std::fs::read_to_string(&prompt_path) {
                    Ok(content) => Ok(content),
                    Err(_) => {
                        warn!("Prompt template '{}' not found, using built-in simple", name);
                        Ok(BUILTIN_SIMPLE.to_string())
                    }
                }
            }
        }
    }
    
    pub fn format_prompt(&self, template: &str, text: &str) -> String {
        template.replace("{text}", &text.replace('"', r#"\""#))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_simple_prompt() {
        let loader = PromptLoader::new().unwrap();
        let prompt = loader.load_prompt(None).unwrap();
        
        assert!(prompt.contains("person_name"));
        assert!(prompt.contains("PII detector"));
        assert!(!prompt.contains("hostname"));
    }

    #[test]
    fn test_builtin_simple_explicit() {
        let loader = PromptLoader::new().unwrap();
        let prompt = loader.load_prompt(Some(&"simple".to_string())).unwrap();
        
        assert!(prompt.contains("PII detector"));
        assert!(!prompt.contains("hostname"));
    }

    #[test]
    fn test_builtin_detailed_prompt() {
        let loader = PromptLoader::new().unwrap();
        let prompt = loader.load_prompt(Some(&"detailed".to_string())).unwrap();
        
        assert!(prompt.contains("person_name"));
        assert!(prompt.contains("hostname"));
        assert!(prompt.contains("node_name"));
        assert!(prompt.contains("Wazuh"));
    }

    #[test]
    fn test_nonexistent_prompt_fallback() {
        let loader = PromptLoader::new().unwrap();
        let prompt = loader.load_prompt(Some(&"nonexistent123".to_string())).unwrap();
        
        assert!(prompt.contains("PII detector"));
    }

    #[test]
    fn test_prompt_formatting() {
        let loader = PromptLoader::new().unwrap();
        let template = "TEXT: \"{text}\" - END";
        let formatted = loader.format_prompt(template, "test@example.com");
        
        assert_eq!(formatted, "TEXT: \"test@example.com\" - END");
    }
}
