use crate::{Data, Error};
use poise::serenity_prelude as serenity;
use serenity::Result;

pub async fn event_handler(
    _ctx: &serenity::Context,
    event: &poise::serenity_prelude::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    _data: &Data,
) -> Result<(), Error> {
    println!(
        "Got an event in event handler: {:?}",
        event.snake_case_name()
    );

    if let poise::serenity_prelude::FullEvent::Message { new_message } = event {
        if new_message.author.bot {
            return Ok(());
        }
    }
    Ok(())
}
