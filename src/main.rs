use serenity::all::Ready;
use serenity::async_trait;
use serenity::client::EventHandler;
use serenity::framework::standard::Configuration;
use serenity::framework::StandardFramework;
use serenity::prelude::*;
use serenity::Client;

use songbird::SerenityInit;
use tracing::info;
use tracing::instrument;
use tracing_subscriber;

mod command;
use command::GENERAL_GROUP;

static CLIENT: RwLock<Option<reqwest::Client>> = RwLock::const_new(None);

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let token = dotenvy::var("DISCORD_TOKEN")?;
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let framework = StandardFramework::new().group(&GENERAL_GROUP);
    framework.configure(
        Configuration::new()
            .prefixes(["]", "}"])
            .case_insensitivity(true),
    );
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
