use serenity::all::Message;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::macros::group;
use serenity::framework::standard::CommandResult;
use serenity::prelude::*;

#[group]
#[commands(ping, join, leave)]
struct General;

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;
    Ok(())
}

#[command]
#[only_in(guilds)]
#[aliases(j)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let manager = songbird::get(&ctx).await.expect("Songbird Not initialized");
    let channel_id = msg
        .guild(&ctx.cache)
        .unwrap()
        .voice_states
        .get(&msg.author.id)
        .and_then(|state| state.channel_id);
    let return_msg = if let Some(c) = channel_id {
        match manager.join(msg.guild_id.unwrap(), c).await {
            Ok(_) => "JOIN!".to_owned(),
            Err(e) => format!("Join failed: {e:?}"),
        }
    } else {
        "Not in a voice channel".to_owned()
    };

    msg.channel_id.say(&ctx.http, return_msg).await?;
    Ok(())
}

#[command]
#[only_in(guilds)]
#[aliases(l)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let manager = songbird::get(&ctx).await.expect("Songbird Not initialized");
    let return_msg = match manager.leave(msg.guild_id.unwrap()).await {
        Ok(_) => "LEAVE!".to_owned(),
        Err(e) => format!("Leave failed: {e:?}"),
    };
    msg.channel_id.say(&ctx.http, return_msg).await?;
    Ok(())
}
