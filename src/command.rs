use std::sync::Arc;

use poise::CreateReply;
use poise::command;

use serenity::all::GuildId;
use serenity::async_trait;
use serenity::builder::CreateEmbed;
use songbird::{Event, TrackEvent, EventHandler, EventContext};

use crate::Context;
use crate::structs::AudioLink;
use crate::structs::Data;
use crate::structs::ParseResult;

/// Show this help menu
#[command(
    prefix_command,
    slash_command,
    track_edits,
    aliases("h"),
    description_localized("zh-TW", "顯示指令幫助清單"),
)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[description_localized("zh-TW", "想要查詢的指令")]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> anyhow::Result<()> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "Just a simple music bot.\nValid prefixes are `]` and `}`, commands are case insensitive.",
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

/// Just a typical command, what do you expect?
#[command(prefix_command, slash_command, description_localized("zh-TW", "你期望什麼呢？"))]
pub async fn ping(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    ctx.say("Pong!").await?;
    Ok(())
}

/// Join the voice channel you are currently in
#[command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("j"),
    description_localized("zh-TW", "加入你所在的語音頻道"),
)]
pub async fn join(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
    let guild_id = ctx.guild_id().expect("Guild only command");
    let channel_id = ctx.guild().unwrap().voice_states
        .get(&ctx.author().id)
        .and_then(|state| state.channel_id);
    let return_msg = if let Some(c) = channel_id {
        match manager.join(guild_id, c).await {
            Ok(call) => {
                call.lock().await
                    .add_global_event(
                        Event::Track(TrackEvent::End),
                        TrackEndNotifier {
                            guild_id,
                            data: ctx.data().clone(),
                            songbird: manager,
                        }
                    );
                "Successfully joined the voice channel!".to_owned()
            },
            Err(e) => format!("Join failed: {e:?}"),
        }
    } else {
        "Not in a voice channel".to_owned()
    };

    ctx.say(return_msg).await?;
    Ok(())
}

/// Leave the voice channel
#[command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("l"),
    description_localized("zh-TW", "離開語音頻道"),
)]
pub async fn leave(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
    let return_msg = match manager.leave(ctx.guild_id().unwrap()).await {
        Ok(_) => "Left the voice channel!".to_owned(),
        Err(e) => format!("Leave failed: {e:?}"),
    };
    ctx.say(return_msg).await?;
    Ok(())
}

/// Play music
#[command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("p"),
    description_localized("zh-TW", "播放音樂"),
)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "The link of the music you want to play"]
    #[description_localized("zh-TW", "想要播放音樂的連結")]
    url: String,
) -> anyhow::Result<()> {
    ctx.defer().await?;
    let guild_id = ctx.guild_id().expect("Guild Only Command");
    let parse_result = AudioLink::parse(&url).await;
    let mut state = ctx.data().get(guild_id);
    match parse_result {
        Ok(ParseResult::Single(audio)) => {
            if state.player.playing {
                state.player.queue.push_back(audio);
                ctx.say("Added to queue!").await?;
            } else {
                state.player.playing = true;
                let msg = format!("Playing `{}`", audio);
                let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
                let call = manager.get_or_insert(guild_id);
                (*call).lock().await.play(audio.into());
                ctx.say(msg).await?;
            }
        },
        Ok(ParseResult::Multiple(audio_list, meta)) => {
            let list_len = audio_list.len();
            state.player.queue.append(&mut audio_list.into());
            if !state.player.playing {
                if let Some(audio) = state.player.queue.pop_front() {
                    state.player.playing = true;
                    let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
                    let call = manager.get_or_insert(guild_id);
                    (*call).lock().await.play(audio.into());
                }
            }
            ctx.say(format!("`{}`\n{} songs added to queue!", meta.title, list_len)).await?;
        },
        Err(_) => {
            ctx.say("Operation failed, no song added").await?;
        },
    };
    Ok(())
}

/// Stop playing songs (clears the play queue)
#[command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("s"),
    description_localized("zh-TW", "停止播放（會清除歌單）"),
)]
pub async fn stop(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let guild_id = ctx.guild_id().expect("Guild Only Command");
    let mut state = ctx.data().get(guild_id);
    state.player.playing = false;
    state.player.queue.clear();
    let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
    let call = manager.get_or_insert(guild_id);
    (*call).lock().await.stop();
    ctx.say("Player stopped!").await?;
    Ok(())
}

/// List songs in the play queue
#[command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("q"),
    description_localized("zh-TW", "顯示歌單"),
)]
pub async fn queue(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let guild_id = ctx.guild_id().expect("Guild Only Command");
    let state = ctx.data().get(guild_id);
    if state.player.queue.len() == 0 {
        ctx.say("There's no song in the queue").await?;
        return Ok(());
    }
    let body = state.player.queue.iter()
        .map(|entry| format!("- `{}` [{}]", entry, entry.time_str()))
        .fold(format!("Total of {} songs:", state.player.queue.len()), |acc, e| acc + "\n" + &e);
    ctx.send(
        CreateReply::default()
        .embed(
            CreateEmbed::new()
            .title("Play Queue")
            .description(body)
        )
    ).await?;
    Ok(())
}

struct TrackEndNotifier {
    guild_id: GuildId,
    data: Data,
    songbird: Arc<songbird::Songbird>,
}

#[async_trait]
impl EventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track([(_track_state, _track_handle)]) = ctx {
            let mut state = self.data.get(self.guild_id);
            if let Some(next_song) = state.player.queue.pop_front() {
                let call = self.songbird.get_or_insert(self.guild_id);
                (*call).lock().await.play(next_song.into());
            } else {
                state.player.playing = false;
            }
        }
        None
    }
}
