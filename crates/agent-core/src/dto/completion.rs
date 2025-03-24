use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct LlmCompletion {
    pub id: String,
    pub model: String,
    pub created: u64,
    pub choices: Vec<LlmCompletionChoice>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LlmCompletionChoice {
    pub index: u32,
    pub finish_reason: LlmCompletionFinishReason,
    pub message: LlmCompletionMessage,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LlmCompletionFinishReason {
    Stop,
    Length,
    ToolCalls,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LlmCompletionMessage {
    pub content: String,
}
