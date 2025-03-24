use crate::{
    dto::{completion::LlmCompletion, completion_chunk::LlmCompletionChunk, input::LlmInput},
    llm_connection::LlmConnection,
    llm_error::LlmError,
};
use async_stream::try_stream;
use futures::{future::BoxFuture, Stream, StreamExt};
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio_util::io::StreamReader;

pub type LlmTextHandler<'a, I, O> = dyn Fn(&I, String) -> BoxFuture<'a, Result<O, LlmError>> + 'a;

pub struct Llm<'a, I, O> {
    client: reqwest::Client,
    text_handler: Option<&'a LlmTextHandler<'a, I, O>>,
}

impl<'a, I, O> Llm<'a, I, O> {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: reqwest::Client::builder()
                .http2_keep_alive_timeout(Duration::from_secs(60))
                .build()?,
            text_handler: None,
        })
    }

    pub fn with_text_handler(
        mut self,
        handler: &'a (impl Fn(&I, String) -> BoxFuture<'a, Result<O, LlmError>> + 'a),
    ) -> Self {
        self.text_handler = Some(handler);
        self
    }

    pub async fn run(
        &self,
        conn: &LlmConnection,
        input: &I,
        body: LlmInput<'_>,
    ) -> Result<O, LlmError> {
        let url = format!("{}/v1/chat/completions", conn.base_url);
        let mut request_builder = self.client.post(url).json(&body);

        if let Some(api_key) = &conn.api_key {
            request_builder = request_builder.bearer_auth(api_key);
        }

        if let Some(options) = &conn.options {
            for (key, value) in &options.headers {
                request_builder = request_builder.header(key, value);
            }
        }

        let response = request_builder.send().await?;
        let response: LlmCompletion = response.json().await?;

        let choice = match response.choices.into_iter().next() {
            Some(choice) => choice,
            None => {
                panic!("no choice found in response");
            }
        };

        match self.text_handler {
            Some(handler) => handler(input, choice.message.content).await,
            None => {
                panic!("no text handler provided");
            }
        }
    }

    pub async fn run_stream(
        &'a self,
        conn: &LlmConnection,
        input: &'a I,
        body: LlmInput<'_>,
    ) -> Result<impl Stream<Item = Result<O, LlmError>> + 'a, LlmError> {
        let url = format!("{}/v1/chat/completions", conn.base_url);
        let mut request_builder = self.client.post(url).json(&body);

        if let Some(api_key) = &conn.api_key {
            request_builder = request_builder.bearer_auth(api_key);
        }

        if let Some(options) = &conn.options {
            for (key, value) in &options.headers {
                request_builder = request_builder.header(key, value);
            }
        }

        let response = request_builder.send().await?;
        let stream = response.bytes_stream();
        let stream_reader = StreamReader::new(stream.map(|result| {
            result.map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
        }));
        let mut lines = stream_reader.lines();

        Ok(try_stream! {
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(chunk) = line.strip_prefix("data: ") {
                    if chunk == "[DONE]" {
                        break;
                    }

                    let chunk = serde_json::from_str::<LlmCompletionChunk>(chunk)?;
                    let choice = match chunk.choices.into_iter().next() {
                        Some(choice) => choice,
                        None => {
                            panic!("no choice found in response");
                        }
                    };
                    let delta = match choice.delta {
                        Some(delta) => delta,
                        None => {
                            continue;
                        }
                    };
                    let content = match delta.content {
                        Some(content) => content,
                        None => {
                            continue;
                        }
                    };

                    match self.text_handler {
                        Some(handler) => yield handler(input, content).await.unwrap(),
                        None => {
                            panic!("no text handler provided");
                        }
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::input::*;
    use futures::{pin_mut, TryStreamExt};

    #[tokio::test]
    async fn test_llm_with_text_handler() {
        let llm_connection = LlmConnection::from_env(None).unwrap();
        let llm_model = std::env::var("LLM_MODEL").unwrap();
        let llm = Llm::new()
            .unwrap()
            .with_text_handler(&|_input, text| Box::pin(async move { Ok(text.trim().to_owned()) }));

        let result = llm
            .run(
                &llm_connection,
                &llm_model,
                LlmInput {
                    model: &llm_model,
                    messages: vec![LlmMessage {
                        role: LlmMessageRole::User,
                        content: vec![LlmMessageContentPart::Text {
                            text: "Hello, world! Can you just say 'Hi', without any other text?",
                        }],
                    }],
                    temperature: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                    parallel_tool_calls: None,
                    stream: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(result, "Hi");
    }

    #[tokio::test]
    async fn test_llm_stream() {
        let llm_connection = LlmConnection::from_env(None).unwrap();
        let llm_model = std::env::var("LLM_MODEL").unwrap();
        let llm = Llm::new()
            .unwrap()
            .with_text_handler(&|_input, text| Box::pin(async move { Ok(text) }));

        let result = llm
            .run_stream(
                &llm_connection,
                &llm_model,
                LlmInput {
                    model: &llm_model,
                    messages: vec![LlmMessage {
                        role: LlmMessageRole::User,
                        content: vec![LlmMessageContentPart::Text {
                            text: "Hello, world!",
                        }],
                    }],
                    temperature: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                    parallel_tool_calls: None,
                    stream: Some(true),
                },
            )
            .await
            .unwrap();

        pin_mut!(result);

        while let Some(result) = result.try_next().await.unwrap() {
            println!("result: {}", result);
        }
    }
}
