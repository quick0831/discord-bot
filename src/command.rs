use std::sync::Arc;

use poise::CreateReply;
use poise::command;

use serenity::all::GuildId;
use serenity::all::ReactionType;
use serenity::async_trait;
use serenity::builder::CreateEmbed;
use songbird::{Event, TrackEvent, EventHandler, EventContext};
use tokio::sync::Mutex;

use crate::Context;
use crate::sources::youtube::search_yt;
use crate::structs::AudioLink;
use crate::structs::Data;
use crate::structs::ParseResult;
use crate::structs::PlayerState;

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
    let guild_id = ctx.guild_id().expect("Guild only command");
    let return_msg = match _join(ctx).await {
        Ok(_) => {
            let mut state = ctx.data().get(guild_id);
            if matches!(state.player.state, PlayerState::Offline) {
                state.player.state = PlayerState::Idle;
            }
            "Successfully joined the voice channel!".to_owned()
        },
        Err(JoinError::Failed(e)) => format!("Join failed: {e:?}"),
        Err(JoinError::NotInChannel) => "Not in a voice channel".to_owned(),
    };
    ctx.say(return_msg).await?;
    Ok(())
}

enum JoinError {
    Failed(songbird::error::JoinError),
    NotInChannel,
}

async fn _join(ctx: Context<'_>) -> Result<Arc<Mutex<songbird::Call>>, JoinError> {
    let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
    let guild_id = ctx.guild_id().expect("Guild only command");
    let channel_id = ctx.guild().unwrap().voice_states
        .get(&ctx.author().id)
        .and_then(|state| state.channel_id);
    if let Some(c) = channel_id {
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
                Ok(call)
            },
            Err(e) => Err(JoinError::Failed(e)),
        }
    } else {
        Err(JoinError::NotInChannel)
    }
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
    let guild_id = ctx.guild_id().expect("Guild Only Command");
    let mut state = ctx.data().get(guild_id);
    state.player.state = PlayerState::Offline;
    state.player.queue.clear();
    let return_msg = match manager.leave(guild_id).await {
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
            match state.player.state {
                PlayerState::Playing => { ctx.say("Added to queue!").await?; },
                _ => { ctx.say(format!("Playing `{}`", audio)).await?; },
            }
            state.player.queue.push_back(audio);
        },
        Ok(ParseResult::Multiple(audio_list, meta)) => {
            ctx.say(format!("`{}`\n{} songs added to queue!", meta.title, audio_list.len())).await?;
            state.player.queue.append(&mut audio_list.into());
        },
        Err(_) => {
            ctx.say("Operation failed, no song added").await?;
        },
    };
    if matches!(state.player.state, PlayerState::Offline) {
        match _join(ctx).await {
            Ok(_) => { state.player.state = PlayerState::Idle },
            Err(JoinError::Failed(e)) => { ctx.say(format!("Join failed: {e:?}")).await?; },
            Err(JoinError::NotInChannel) => { ctx.say("Not in a voice channel").await?; },
        }
    }
    if !matches!(state.player.state, PlayerState::Playing) {
        if let Some(audio) = state.player.queue.pop_front() {
            state.player.state = PlayerState::Playing;
            let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
            let call = manager.get_or_insert(guild_id);
            (*call).lock().await.play(audio.into());
        }
    }
    Ok(())
}

/// Search on Youtube
#[command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("se"),
    description_localized("zh-TW", "搜尋Youtube"),
)]
pub async fn search(
    ctx: Context<'_>,
    #[description = "The search prompt"]
    #[description_localized("zh-TW", "想要搜尋的關鍵字")]
    #[rest]
    prompt: String,
) -> anyhow::Result<()> {
    ctx.defer().await?;
    let guild_id = ctx.guild_id().expect("Guild Only Command");
    let search_result = search_yt(&prompt).await?;

    let emoji_str = ["one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "keycap_ten"];
    let body = search_result.iter().zip(emoji_str)
        .map(|(info, e)| format!(":{e}: `{}` [{}:{:02}]", info.title, info.duration / 60, info.duration % 60))
        .fold(format!("Here are the results:"), |acc, e| acc + "\n" + &e);
    let handle = ctx.send(
        CreateReply::default()
        .embed(
            CreateEmbed::new()
            .title("Search Result")
            .description(body)
        )
    ).await?;
    handle.into_message().await?
        .react(&ctx.http(), ReactionType::try_from("1️⃣")?).await?;

    let mut state = ctx.data().get(guild_id);
    if matches!(state.player.state, PlayerState::Offline) {
        match _join(ctx).await {
            Ok(_) => { state.player.state = PlayerState::Idle },
            Err(JoinError::Failed(e)) => { ctx.say(format!("Join failed: {e:?}")).await?; },
            Err(JoinError::NotInChannel) => { ctx.say("Not in a voice channel").await?; },
        }
    }
    let index = 0;
    let audio = search_result.into_iter().nth(index).expect("index in range").into();
    if matches!(state.player.state, PlayerState::Playing) {
        ctx.say("Added to queue!").await?;
        state.player.queue.push_back(audio);
    } else if matches!(state.player.state, PlayerState::Idle) {
        ctx.say(format!("Playing `{}`", audio)).await?;
        state.player.state = PlayerState::Playing;
        let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
        let call = manager.get_or_insert(guild_id);
        (*call).lock().await.play(audio.into());
    }
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
    let msg = match state.player.state {
        PlayerState::Offline => "The bot is not in a voice channel!",
        _ => "Player stopped!",
    };
    state.player.state = PlayerState::Idle;
    state.player.queue.clear();
    let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
    let call = manager.get_or_insert(guild_id);
    (*call).lock().await.stop();
    ctx.say(msg).await?;
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
                state.player.state = PlayerState::Idle;
            }
        }
        None
    }
}
