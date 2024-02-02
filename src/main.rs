use serenity::all::Ready;
use serenity::async_trait;
use serenity::framework::StandardFramework;
use serenity::framework::standard::Configuration;
use serenity::prelude::*;
use serenity::Client;
use serenity::client::EventHandler;

use songbird::SerenityInit;
use tracing::info;
use tracing::instrument;
use tracing_subscriber;

mod command;
use command::GENERAL_GROUP;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let token = dotenvy::var("DISCORD_TOKEN")?;
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT;
    let framework = StandardFramework::new().group(&GENERAL_GROUP);
    framework.configure(Configuration::new().prefix("]"));
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
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("{} is Ready!", ready.user.name);
    }
}
