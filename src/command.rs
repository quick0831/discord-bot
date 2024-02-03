use std::collections::HashSet;

use serenity::all::Message;
use serenity::all::UserId;
use serenity::framework::standard::{
    Args,
    CommandGroup,
    CommandResult,
    HelpOptions,
    help_commands,
    macros::command,
    macros::group,
    macros::help,
};
use serenity::prelude::*;
use songbird::input::YoutubeDl;

use crate::get_client;

#[help]
async fn help_command(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[group]
#[commands(ping, join, leave, play)]
struct General;

#[command]
#[description = "As typical as it was"]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;
    Ok(())
}

#[command]
#[only_in(guilds)]
#[aliases(j)]
#[description = "Join the voice channel you are in"]
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
#[description = "Leave the voice channel"]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let manager = songbird::get(&ctx).await.expect("Songbird Not initialized");
    let return_msg = match manager.leave(msg.guild_id.unwrap()).await {
        Ok(_) => "LEAVE!".to_owned(),
        Err(e) => format!("Leave failed: {e:?}"),
    };
    msg.channel_id.say(&ctx.http, return_msg).await?;
    Ok(())
}

#[command]
#[only_in(guilds)]
#[aliases(p)]
#[description = "Play a song"]
async fn play(ctx: &Context, msg: &Message) -> CommandResult {
    let manager = songbird::get(&ctx).await.expect("Songbird Not initialized");
    let client = get_client().await;
    let url = "https://www.youtube.com/watch?v=i8OUh3YvRpk".to_string();
    let f = YoutubeDl::new(client, url);
    let call = manager.get_or_insert(msg.guild_id.expect("Command in Guild Channel"));
    (*call).lock().await.play(f.into());
    msg.channel_id.say(&ctx.http, "Play!").await?;
    Ok(())
}
