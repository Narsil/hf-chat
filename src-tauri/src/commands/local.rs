use std::sync::Arc;

use crate::entities::message;
use mistralrs::{
    Model, NormalRequest, PagedAttentionMetaBuilder, Request, RequestLike, Response,
    TextMessageRole, TextMessages, TextModelBuilder,
};
use tauri::async_runtime::{channel, Receiver};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Default(#[from] anyhow::Error),

    #[error(transparent)]
    Mpsc(#[from] tokio::sync::mpsc::error::SendError<mistralrs::Request>),

    #[error(transparent)]
    MistralRs(#[from] mistralrs::MistralRsError),
}

pub struct Stream {
    model: Model,
    rx: Receiver<Response>,
}

impl Stream {
    pub async fn next(&mut self) -> Option<String> {
        let chunk = self.rx.recv().await?;
        if let Response::Chunk(chunk) = chunk {
            Some(chunk.choices[0].delta.content.to_string())
        } else {
            None
        }
    }
}

fn to_mistralrs(messages: Vec<message::Model>) -> TextMessages {
    let mut newmessages = TextMessages::new();
    let mut role = TextMessageRole::Assistant;
    let mut last_user = None;
    let mut last_message: Option<String> = None;
    for message in messages {
        if Some(message.user_id) != last_user {
            if let Some(last_message) = last_message.take() {
                newmessages = newmessages.add_message(role.clone(), last_message);
            }
            role = match role {
                TextMessageRole::Custom(_) => TextMessageRole::User,
                TextMessageRole::Tool => TextMessageRole::User,
                TextMessageRole::System => TextMessageRole::User,
                TextMessageRole::User => TextMessageRole::Assistant,
                TextMessageRole::Assistant => TextMessageRole::User,
            };
            last_message = Some(message.content.clone());
            last_user = Some(message.user_id);
        } else {
            last_message.as_mut().map(|m| {
                m.push('\n');
                m.push_str(&message.content);
            });
        }
    }
    if let Some(last_message) = last_message.take() {
        newmessages = newmessages.add_message(role, last_message);
    }
    newmessages
}

pub async fn local_stream(
    model_id: String,
    messages: Vec<message::Model>,
) -> Result<Stream, Error> {
    let model = TextModelBuilder::new(model_id)
        // .with_isq(IsqType::Q8_0)
        // .with_logging()
        .with_paged_attn(|| PagedAttentionMetaBuilder::default().build())?
        .build()
        .await?;

    log::info!("Model started");
    let messages = to_mistralrs(messages);
    log::info!("Conversation {messages:?}");

    let (tx, rx) = channel(20);

    let mut request = messages;
    let (tools, tool_choice) = if let Some((a, b)) = request.take_tools() {
        (Some(a), Some(b))
    } else {
        (None, None)
    };
    let request = Request::Normal(NormalRequest {
        messages: request.take_messages(),
        sampling_params: request.take_sampling_params(),
        response: tx,
        return_logprobs: request.return_logprobs(),
        is_streaming: true,
        id: 0,
        constraint: request.take_constraint(),
        suffix: None,
        adapters: request.take_adapters(),
        tools,
        tool_choice,
        logits_processors: request.take_logits_processors(),
        return_raw_logits: false,
    });

    // let model = Arc::new(model);
    model
        .inner()
        .get_sender()
        .unwrap()
        .send(request)
        .await
        .unwrap();

    Ok(Stream { model, rx })
    // while let Some(chunk) = stream.next().await {
    //     if let Response::Chunk(chunk) = chunk {
    //         print!("{}", chunk.choices[0].delta.content);
    //     }
    // }
}
