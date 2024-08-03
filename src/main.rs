mod eventhandler;
mod watcher;

use poise::serenity_prelude as serenity;
use std::{
    env::var,
    sync::{mpsc, Arc},
    time::Duration,
};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
#[derive(Debug)]
pub struct Data {}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let notification_channel_id = match var("NOTIFICATION_CHANNEL_ID") {
        Ok(id) => poise::serenity_prelude::ChannelId::new(id.parse().unwrap()),
        Err(_) => panic!("Missing `NOTIFICATION_CHANNEL_ID` env var"),
    };

    let commands = vec![];

    // FrameworkOptions contains all of poise's configuration option in one struct
    // Every option can be omitted to use its default value
    let options = poise::FrameworkOptions {
        commands,
        manual_cooldowns: true,
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("~".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(3600),
            ))),
            additional_prefixes: vec![
                poise::Prefix::Literal("hey bot"),
                poise::Prefix::Literal("hey bot,"),
            ],
            ..Default::default()
        },
        // The global error handler for all error cases that may occur
        on_error: |error| Box::pin(on_error(error)),
        // This code is run before every command
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        // This code is run after a command if it was successful (returned Ok)
        post_command: |ctx| {
            Box::pin(async move {
                println!(
                    "Executed command {} in {}!",
                    ctx.command().qualified_name,
                    ctx.channel_id()
                );
            })
        },
        // Every command invocation must pass this check to continue execution
        command_check: Some(|ctx| {
            Box::pin(async move {
                if ctx.author().id == 123456789 {
                    return Ok(false);
                }

                Ok(true)
            })
        }),
        // Enforce command checks even for owners (enforced by default)
        // Set to true to bypass checks, which is useful for testing
        skip_checks_for_owners: false,
        event_handler: |ctx, event, _framework, data| {
            Box::pin(eventhandler::event_handler(ctx, event, _framework, data))
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |_ctx, _ready, _framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                Ok(Data {})
            })
        })
        .options(options)
        .build();

    let token = var("DISCORD_TOKEN")
        .expect("Missing `DISCORD_TOKEN` env var, see README for more information.");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    let sender = client.as_ref().unwrap().http.clone();
    let (tx, rx) = mpsc::channel();
    let app = watcher::Executor::new(notification_channel_id, sender);
    tokio::spawn(async move {
        app.start(rx).await;
    });

    let shard_manager = client.as_ref().unwrap().shard_manager.clone();

    tokio::spawn(async move {
        client.unwrap().start().await.unwrap();
    });
    wait_until_shutdown().await;
    let _ = tx.send(());
    shard_manager.shutdown_all().await;
}

#[cfg(unix)]
async fn wait_until_shutdown() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sighup = signal(SignalKind::hangup()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        v = sigint.recv() => {
            println!("Received A SIGINT, shutting down...");
            v.unwrap()
        },
        v = sigterm.recv() => {
            println!("Received SIGTERM, shutting down...");
            v.unwrap()
        }
        v = sighup.recv() => {
            println!("Received SIGHUP, shutting down...");
            v.unwrap()
        }
    }
}

#[cfg(windows)]
async fn wait_until_shutdown() {
    use tokio::signal::windows::{signal, SignalKind};
    tokio::signal::ctrl_c().await.unwrap();
    println!("Received CTRL-C, shutting down...");
}
