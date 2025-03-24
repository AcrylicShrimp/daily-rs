use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, env::VarError};

#[derive(Serialize, Deserialize, Debug)]
pub struct LlmConnection {
    pub base_url: String,
    pub api_key: Option<String>,
    pub options: Option<LlmConnectionOptions>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LlmConnectionOptions {
    pub headers: BTreeMap<String, String>,
}

impl LlmConnection {
    pub fn from_env(headers: Option<BTreeMap<String, String>>) -> Result<Self, VarError> {
        let base_url = std::env::var("LLM_BASE_URL")?;
        let api_key = std::env::var("LLM_API_KEY").ok().and_then(|key| {
            let key = key.trim();

            if key.is_empty() {
                None
            } else {
                Some(key.to_owned())
            }
        });

        Ok(Self {
            base_url,
            api_key,
            options: headers.map(|headers| LlmConnectionOptions { headers }),
        })
    }
}
