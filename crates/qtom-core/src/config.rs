use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectConfig {
    pub openai_api_key_present: bool,
    pub evaluator_model: String,
    pub local_model: String,
    pub default_k: usize,
    pub default_agent_count: usize,
}

impl ProjectConfig {
    pub fn from_env_and_dotenv(path: impl AsRef<Path>) -> Self {
        let dotenv = read_dotenv(path);

        Self {
            openai_api_key_present: value_for("OPENAI_API_KEY", &dotenv)
                .is_some_and(|value| !value.trim().is_empty()),
            evaluator_model: value_for("QTOM_EVALUATOR_MODEL", &dotenv)
                .unwrap_or_else(|| "gpt-5.5-medium".to_string()),
            local_model: value_for("QTOM_LOCAL_MODEL", &dotenv)
                .unwrap_or_else(|| "Qwen3-2507".to_string()),
            default_k: value_for("QTOM_DEFAULT_K", &dotenv)
                .and_then(|value| value.parse().ok())
                .unwrap_or(8),
            default_agent_count: value_for("QTOM_DEFAULT_AGENT_COUNT", &dotenv)
                .and_then(|value| value.parse().ok())
                .unwrap_or(128),
        }
    }
}

fn value_for(key: &str, dotenv: &HashMap<String, String>) -> Option<String> {
    std::env::var(key).ok().or_else(|| dotenv.get(key).cloned())
}

fn read_dotenv(path: impl AsRef<Path>) -> HashMap<String, String> {
    let Ok(contents) = fs::read_to_string(path) else {
        return HashMap::new();
    };

    contents
        .lines()
        .filter_map(parse_dotenv_line)
        .collect::<HashMap<_, _>>()
}

fn parse_dotenv_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let (key, value) = trimmed.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }

    Some((key.to_string(), unquote(value.trim())))
}

fn unquote(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes[0];
        let last = bytes[value.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return value[1..value.len() - 1].to_string();
        }
    }

    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dotenv_without_leaking_secret_value() {
        let dir = std::env::temp_dir().join(format!(
            "qtom-env-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".env");
        let contents = [
            format!("{}=test-token-present", "OPENAI_API_KEY"),
            "QTOM_EVALUATOR_MODEL=test-evaluator".to_string(),
            "QTOM_LOCAL_MODEL=Qwen3-2507".to_string(),
            "QTOM_DEFAULT_K=4".to_string(),
            "QTOM_DEFAULT_AGENT_COUNT=1024".to_string(),
        ]
        .join("\n");
        fs::write(&path, contents).unwrap();

        let config = ProjectConfig::from_env_and_dotenv(&path);

        assert!(config.openai_api_key_present);
        assert_eq!(config.evaluator_model, "test-evaluator");
        assert_eq!(config.local_model, "Qwen3-2507");
        assert_eq!(config.default_k, 4);
        assert_eq!(config.default_agent_count, 1024);

        let _ = fs::remove_file(path);
        let _ = fs::remove_dir(dir);
    }
}
