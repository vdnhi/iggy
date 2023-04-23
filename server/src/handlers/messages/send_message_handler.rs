use crate::handlers::STATUS_OK;
use anyhow::Result;
use shared::error::Error;
use shared::messages::send_message::SendMessage;
use streaming::message::Message;
use streaming::system::System;
use tracing::trace;

pub async fn handle(
    command: SendMessage,
    send: &mut quinn::SendStream,
    system: &mut System,
) -> Result<(), Error> {
    trace!(
        "Appending message to stream: {:?}, topic: {:?}, key kind: {:?}, key value: {:?}, payload: {:?}",
        command.stream_id, command.topic_id, command.key_kind, command.key_value, command.payload
    );

    let message = Message::empty(command.payload);
    system
        .get_stream_mut(command.stream_id)?
        .append_messages(command.topic_id, command.key_value, message)
        .await?;
    send.write_all(STATUS_OK).await?;
    Ok(())
}