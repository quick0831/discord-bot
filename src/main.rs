use serenity::all::Message;
use serenity::all::Ready;
use serenity::async_trait;
use serenity::prelude::*;
use serenity::Client;
use serenity::client::EventHandler;

use songbird::SerenityInit;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let token = dotenvy::var("DISCORD_TOKEN")?;
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
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

    #[instrument(skip_all, fields(msg.content))]
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot { return; }
        if msg.channel_id.get() != 1202687853424017458 { return; }
        
        if matches!(msg.content.get(..1), Some("]") | Some("}")) {
            let s = msg.content[1..].split(" ").collect::<Vec<_>>();
            let cmd = s[0].to_lowercase();
            let ret_msg = if matches!(cmd.as_str(), "j" | "join") {
                let manager = songbird::get(&ctx).await.expect("Songbird Not initialized");
                let channel_id = msg.guild(&ctx.cache).unwrap()
                    .voice_states.get(&msg.author.id)
                    .and_then(|state| state.channel_id);
                if let Some(c) = channel_id {
                    match manager.join(msg.guild_id.unwrap(), c).await {
                        Ok(_) => "JOIN!".to_owned(),
                        Err(e) => format!("Join failed: {e:?}")
                    }
                } else {
                    "Not in a voice channel".to_owned()
                }
            } else if matches!(cmd.as_str(), "l" | "leave") {
                let manager = songbird::get(&ctx).await.expect("Songbird Not initialized");
                match manager.leave(msg.guild_id.unwrap()).await {
                    Ok(_) => "LEAVE!".to_owned(),
                    Err(e) => format!("Leave failed: {e:?}")
                }
            } else {
                format!("Command received: {}\nDebug: {:?}", s[0], s)
            };
            if let Err(e) = msg.channel_id.say(&ctx.http, ret_msg).await {
                error!("{e}");
            }
        }
    }
}
