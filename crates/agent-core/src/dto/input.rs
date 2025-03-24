use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct LlmInput<'a> {
    pub model: &'a str,
    pub messages: Vec<LlmMessage<'a>>,
    pub temperature: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub parallel_tool_calls: Option<bool>,
    pub stream: Option<bool>,
}

#[derive(Serialize, Debug, Clone)]
pub struct LlmMessage<'a> {
    pub role: LlmMessageRole,
    pub content: Vec<LlmMessageContentPart<'a>>,
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LlmMessageRole {
    Developer,
    System,
    User,
    Assistant,
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum LlmMessageContentPart<'a> {
    Text {
        text: &'a str,
    },
    Image {
        image_url: LlmMessageContentPartImageUrl<'a>,
    },
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LlmMessageContentPartImageUrl<'a> {
    pub url: &'a str,
}
