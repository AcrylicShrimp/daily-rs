use agent_core::{
    dto::input::{LlmInput, LlmMessage, LlmMessageContentPart, LlmMessageRole},
    llm::Llm,
    llm_connection::LlmConnection,
};
use futures::{pin_mut, TryStreamExt};
use std::io::{BufRead, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let llm_connection = LlmConnection::from_env(None).unwrap();
    let llm_model = std::env::var("LLM_MODEL").unwrap();
    let llm = Llm::new()?.with_text_handler(&|_input, text| Box::pin(async move { Ok(text) }));

    struct ConversationHistory {
        query: String,
        answer: String,
    }
    let mut conversation_history = Vec::<ConversationHistory>::new();

    loop {
        print!("Query: ");
        std::io::stdout().flush().unwrap();
        let line = read_line();

        if line.is_empty() {
            continue;
        }

        let mut messages = Vec::with_capacity(conversation_history.len() + 1);

        for message in &conversation_history {
            messages.push(LlmMessage {
                role: LlmMessageRole::User,
                content: vec![LlmMessageContentPart::Text {
                    text: &message.query,
                }],
            });
            messages.push(LlmMessage {
                role: LlmMessageRole::Assistant,
                content: vec![LlmMessageContentPart::Text {
                    text: &message.answer,
                }],
            });
        }

        messages.push(LlmMessage {
            role: LlmMessageRole::User,
            content: vec![LlmMessageContentPart::Text { text: &line }],
        });

        let response = llm
            .run_stream(
                &llm_connection,
                &llm_model,
                LlmInput {
                    model: &llm_model,
                    messages,
                    temperature: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                    parallel_tool_calls: None,
                    stream: Some(true),
                },
            )
            .await
            .unwrap();

        pin_mut!(response);

        print!("Answer: ");
        std::io::stdout().flush().unwrap();

        let mut answer = String::new();

        while let Some(result) = response.try_next().await? {
            print!("{}", result);
            std::io::stdout().flush().unwrap();
            answer.push_str(&result);
        }

        println!("\n");

        conversation_history.push(ConversationHistory {
            query: line,
            answer,
        });
    }
}

fn read_line() -> String {
    let mut line = String::new();
    let stdin = std::io::stdin();
    stdin.lock().read_line(&mut line).unwrap();
    line.trim().to_owned()
}
