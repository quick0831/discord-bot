use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serenity::all::Ready;
use serenity::async_trait;
use serenity::client::EventHandler;
use serenity::prelude::*;
use serenity::Client;

use songbird::SerenityInit;

use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing_subscriber;

mod command;

static CLIENT: RwLock<Option<reqwest::Client>> = RwLock::const_new(None);

pub struct Data {
    votes: Mutex<HashMap<String, u32>>,
}
type Context<'a> = poise::Context<'a, Data, anyhow::Error>;

async fn get_client() -> reqwest::Client {
    {
        let c = CLIENT.read().await;
        if let Some(ref client) = *c {
            return client.clone();
        }
    }
    let client = reqwest::Client::new();
    *CLIENT.write().await = Some(client.clone());
    client
}

async fn on_error(error: poise::FrameworkError<'_, Data, anyhow::Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            error!("Error in command `{}`: {:?}", ctx.command().name, error);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let token = dotenvy::var("DISCORD_TOKEN")?;

    let options = poise::FrameworkOptions {
        commands: vec![
            command::help(),
            command::ping(),
            command::join(),
            command::leave(),
            command::play(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("]".into()),
            additional_prefixes: vec![
                poise::Prefix::Literal("}"),
            ],
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(3600),
            ))),
            ..Default::default()
        },
        on_error: |err| Box::pin(on_error(err)),
        pre_command: |ctx| {
            Box::pin(async move {
                info!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        event_handler: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                info!(
                    "Got an event in event handler: {:?}",
                    event.snake_case_name()
                );
                Ok(())
            })
        },
        ..Default::default()
    };
    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                info!("Logged in as {}", ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    votes: Mutex::new(HashMap::new()),
                })
            })
        })
        .options(options)
        .build();

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await?;
    client.start().await?;
    Ok(())
}

#[derive(Debug)]
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    #[instrument(skip_all)]
    async fn ready(&self, _ctx: serenity::prelude::Context, ready: Ready) {
        info!("{} is Ready!", ready.user.name);
    }
}
