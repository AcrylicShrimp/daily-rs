use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct LlmCompletionChunk {
    pub id: String,
    pub model: String,
    pub created: u64,
    pub choices: Vec<LlmCompletionChunkChoice>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LlmCompletionChunkChoice {
    pub index: u32,
    pub finish_reason: Option<LlmCompletionChunkFinishReason>,
    pub delta: Option<LlmCompletionChunkDelta>,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LlmCompletionChunkFinishReason {
    Stop,
    Length,
    ToolCalls,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LlmCompletionChunkDelta {
    pub content: Option<String>,
}
