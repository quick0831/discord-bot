use poise::command;

use crate::Context;
use crate::structs::AudioLink;

/// Show this help menu
#[command(prefix_command, slash_command, track_edits)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> anyhow::Result<()> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "This is an example bot made to showcase features of my custom Discord bot framework",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

/// As typical as it was
#[command(prefix_command, slash_command)]
pub async fn ping(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    ctx.say("Pong!").await?;
    Ok(())
}

/// Join the voice channel you are in
#[command(prefix_command, slash_command, guild_only)]
pub async fn join(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
    let channel_id = ctx
        .guild()
        .unwrap()
        .voice_states
        .get(&ctx.author().id)
        .and_then(|state| state.channel_id);
    let return_msg = if let Some(c) = channel_id {
        match manager.join(ctx.guild_id().unwrap(), c).await {
            Ok(_) => "JOIN!".to_owned(),
            Err(e) => format!("Join failed: {e:?}"),
        }
    } else {
        "Not in a voice channel".to_owned()
    };

    ctx.say(return_msg).await?;
    Ok(())
}

/// Leave the voice channel
#[command(prefix_command, slash_command, guild_only)]
pub async fn leave(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
    let return_msg = match manager.leave(ctx.guild_id().unwrap()).await {
        Ok(_) => "LEAVE!".to_owned(),
        Err(e) => format!("Leave failed: {e:?}"),
    };
    ctx.say(return_msg).await?;
    Ok(())
}

/// Play a song
#[command(prefix_command, slash_command, guild_only)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "The Youtube link you want to play"]
    #[description_localized("zh-TW", "想要播放的Youtube連結")]
    url: String,
) -> anyhow::Result<()> {
    let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
    let audio = AudioLink::parse(&url).unwrap();
    let call = manager.get_or_insert(ctx.guild_id().unwrap());
    (*call).lock().await.play(audio.into());
    ctx.say("Play!").await?;
    Ok(())
}

/// Stop playing songs
#[command(prefix_command, slash_command, guild_only)]
pub async fn stop(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
    let call = manager.get_or_insert(ctx.guild_id().unwrap());
    (*call).lock().await.stop();
    ctx.say("Stop!").await?;
    Ok(())
}
