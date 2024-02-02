use serenity::all::Message;
use serenity::all::Ready;
use serenity::async_trait;
use serenity::prelude::*;
use serenity::Client;
use serenity::client::EventHandler;

use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let token = dotenvy::var("DISCORD_TOKEN")?;
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
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

    #[instrument(skip_all, fields(msg.content))]
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot { return; }
        if msg.channel_id.get() != 1202687853424017458 { return; }
        
        if matches!(msg.content.get(..1), Some("]") | Some("}")) {
            let s = msg.content[1..].split(" ").collect::<Vec<_>>();
            let ret_msg = format!("Command received: {}\nDebug: {:?}", s[0], s);
            if let Err(e) = msg.channel_id.say(&ctx.http, ret_msg).await {
                error!("{e}");
            }
        }
    }
}
