use std::collections::VecDeque;
use std::mem::replace;
use std::sync::Arc;

use base64::prelude::*;

use poise::CreateReply;
use poise::command;

use serenity::all::GuildId;
use serenity::async_trait;
use serenity::builder::CreateEmbed;

use serenity::builder::CreateEmbedAuthor;
use songbird::{Event, TrackEvent, EventHandler, EventContext};

use tokio::sync::Mutex;

use crate::Context;
use crate::sources::youtube::search_yt;
use crate::structs::AudioLink;
use crate::structs::Data;
use crate::structs::LoopPolicy;
use crate::structs::ParseResult;
use crate::structs::PlayerState;
use crate::structs::UnloadedAudioLink;

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
    if let Some(call) = manager.get(guild_id) {
        (*call).lock().await.stop();
    }
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
                PlayerState::Playing(_) => { ctx.say("Added to queue!").await?; },
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
    if !matches!(state.player.state, PlayerState::Playing(_)) {
        if let Some(audio) = state.player.queue.pop_front() {
            state.player.state = PlayerState::Playing(audio.clone());
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
    let user_id = ctx.author().id;
    let search_result = search_yt(&prompt).await?;

    let emoji_str = ["one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "keycap_ten"];
    let body = search_result.iter().zip(emoji_str)
        .map(|(info, e)| format!(":{e}: `{}` [{}:{:02}]", info.title, info.duration / 60, info.duration % 60))
        .fold("Use `]select <num>`, `/select <num>` or `]n <num>` to select:".to_string(), |acc, e| acc + "\n" + &e);
    let list = search_result.into_iter().map(AudioLink::from).collect::<Vec<_>>();
    let mut state = ctx.data().get(guild_id);
    state.player.search_item.insert(user_id, list);
    ctx.send(
        CreateReply::default()
        .embed(
            CreateEmbed::new()
            .title("Search Result")
            .description(body)
        )
    ).await?;

    Ok(())
}

/// Select from a set of options
#[command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("n"),
    description_localized("zh-TW", "選擇一個選項"),
)]
pub async fn select(
    ctx: Context<'_>,
    #[description = "The index of the option"]
    #[description_localized("zh-TW", "所選擇的編號")]
    index: usize,
) -> anyhow::Result<()> {
    let guild_id = ctx.guild_id().expect("Guild Only Command");
    let user_id = ctx.author().id;
    let mut state = ctx.data().get(guild_id);
    let entry = state.player.search_item.entry(user_id);

    if let std::collections::hash_map::Entry::Occupied(entry) = entry {
        if index != 0 && index <= entry.get().len() {
            let list = entry.remove();
            if matches!(state.player.state, PlayerState::Offline) {
                match _join(ctx).await {
                    Ok(_) => { state.player.state = PlayerState::Idle },
                    Err(JoinError::Failed(e)) => { ctx.say(format!("Join failed: {e:?}")).await?; },
                    Err(JoinError::NotInChannel) => { ctx.say("Not in a voice channel").await?; },
                }
            }
            let audio = list.into_iter().nth(index - 1).expect("index in range").into();
            if matches!(state.player.state, PlayerState::Playing(_)) {
                ctx.say("Added to queue!").await?;
                state.player.queue.push_back(audio);
            } else if matches!(state.player.state, PlayerState::Idle) {
                ctx.say(format!("Playing `{}`", audio)).await?;
                state.player.state = PlayerState::Playing(audio.clone());
                let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
                let call = manager.get_or_insert(guild_id);
                (*call).lock().await.play(audio.into());
            }
        } else {
            ctx.say("Input not in range").await?;
        }
    } else {
        ctx.say("Nothing currently in selection").await?;
    }

    Ok(())
}

/// Stop playing songs (clears the play queue)
#[command(
    prefix_command,
    slash_command,
    guild_only,
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

/// Skip a song
#[command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("s"),
    description_localized("zh-TW", "跳過一首歌曲"),
)]
pub async fn skip(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let guild_id = ctx.guild_id().expect("Guild Only Command");
    let mut state = ctx.data().get(guild_id);
    let msg = match state.player.state {
        PlayerState::Offline => "The bot is not in a voice channel!",
        PlayerState::Idle => "The bot is not currently playing anything!",
        PlayerState::Playing(_) => {
            let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
            let call = manager.get_or_insert(guild_id);
            let mut call = (*call).lock().await;
            call.stop();
            if let Some(audio) = state.player.queue.pop_front() {
                state.player.state = PlayerState::Playing(audio.clone());
                call.play(audio.into());
            } else {
                state.player.state = PlayerState::Idle;
            }
            "Skiped a song!"
        },
    };
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

/// Show the info of the song currently on play
#[command(
    prefix_command,
    slash_command,
    guild_only,
    aliases("np"),
    description_localized("zh-TW", "顯示正在播放歌曲的資訊"),
)]
pub async fn now_playing(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let guild_id = ctx.guild_id().expect("Guild Only Command");
    let state = ctx.data().get(guild_id);
    if let PlayerState::Playing(ref audio) = state.player.state {
        let embed = match audio {
            AudioLink::Youtube(info) => {
                let mut m = CreateEmbed::new()
                    .title(&info.title)
                    .url(format!("https://www.youtube.com/watch?v={}", info.id))
                    .field("Channel", &info.channel, true)
                    .field("Duration", audio.time_str(), true)
                    .author(CreateEmbedAuthor::new("Audio source from Youtube"));
                if let Some(desc) = &info.description {
                    m = m.description(desc);
                }
                if let Some(playlist) = &info.playlist {
                    m = m.field("Playlist", playlist, true);
                }
                m.field("Channel URL", &info.channel_url, false)
            },
        };
        ctx.send(CreateReply::default().embed(embed)).await?;
    } else {
        ctx.say("The player is currently not playing anything!").await?;
    }
    Ok(())
}

/// Set the loop mode of the queue
#[command(
    prefix_command,
    slash_command,
    guild_only,
    rename = "loop",
    description_localized("zh-TW", "設定重複播放模式"),
)]
pub async fn cmd_loop(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let guild_id = ctx.guild_id().expect("Guild Only Command");
    let mut state = ctx.data().get(guild_id);
    state.player.loop_policy = match state.player.loop_policy {
        LoopPolicy::Normal => LoopPolicy::Loop,
        _ => LoopPolicy::Normal,
    };
    let msg = match state.player.loop_policy {
        LoopPolicy::Normal  => "Mode changed to `Normal`!",
        LoopPolicy::Loop    => "Mode changed to `Loop`!",
        LoopPolicy::Random  => "Mode changed to `Random`!",
    };
    ctx.say(msg).await?;
    Ok(())
}

/// Import the play queue
#[command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("zh-TW", "匯入播放佇列"),
)]
pub async fn import(
    ctx: Context<'_>,
    #[description = "Input data"]
    #[description_localized("zh-TW", "輸入資料")]
    input: String,
) -> anyhow::Result<()> {
    let guild_id = ctx.guild_id().expect("Guild Only Command");
    let mut state = ctx.data().get(guild_id);
    let bin = BASE64_STANDARD.decode(input)?;
    let queue: Vec<UnloadedAudioLink> = serde_cbor::from_slice(&bin)?;
    ctx.say(format!("Added {} songs!", queue.len())).await?;
    let handles = queue.into_iter()
        .map(UnloadedAudioLink::load)
        .map(tokio::task::spawn);
    for handle in handles {
        state.player.queue.push_back(handle.await??);
    }
    ctx.say("Done loading").await?;
    if matches!(state.player.state, PlayerState::Offline) {
        match _join(ctx).await {
            Ok(_) => { state.player.state = PlayerState::Idle },
            Err(JoinError::Failed(e)) => { ctx.say(format!("Join failed: {e:?}")).await?; },
            Err(JoinError::NotInChannel) => { ctx.say("Not in a voice channel").await?; },
        }
    }
    if !matches!(state.player.state, PlayerState::Playing(_)) {
        if let Some(audio) = state.player.queue.pop_front() {
            state.player.state = PlayerState::Playing(audio.clone());
            let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
            let call = manager.get_or_insert(guild_id);
            (*call).lock().await.play(audio.into());
        }
    }
    Ok(())
}

/// Export the play queue
#[command(
    prefix_command,
    slash_command,
    guild_only,
    description_localized("zh-TW", "匯出播放佇列"),
)]
pub async fn export(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let guild_id = ctx.guild_id().expect("Guild Only Command");
    let state = ctx.data().get(guild_id);
    let mut value = state.player.queue.iter().map(AudioLink::unload).collect::<VecDeque<_>>();
    if let PlayerState::Playing(ref audio) = state.player.state {
        value.push_front(audio.unload());
    }
    let output = serde_cbor::to_vec(&value)?;
    let output = BASE64_STANDARD.encode(output);
    ctx.say(format!("The exported queue:\n`{output}`")).await?;
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
        if let EventContext::Track(_) = ctx {
            let mut state = self.data.get(self.guild_id);
            let next_state = if let Some(next_song) = state.player.queue.pop_front() {
                let call = self.songbird.get_or_insert(self.guild_id);
                (*call).lock().await.play(next_song.clone().into());
                PlayerState::Playing(next_song)
            } else {
                PlayerState::Idle
            };
            let prev_state = replace(&mut state.player.state, next_state);
            if let PlayerState::Playing(audio) = prev_state {
                match state.player.loop_policy {
                    LoopPolicy::Normal => {},
                    LoopPolicy::Loop => {
                        state.player.queue.push_back(audio.clone());
                    },
                    LoopPolicy::Random => {},
                }
            }
        }
        None
    }
}
