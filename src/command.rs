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
use crate::structs::QueueState;

/// Show this help menu
#[command(prefix_command, slash_command, track_edits, aliases("h"))]
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
#[command(prefix_command, slash_command, guild_only, aliases("j"))]
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
            Ok(call) => {
                call.lock().await
                    .add_global_event(
                        Event::Track(TrackEvent::End),
                        TrackEndNotifier {
                            guild_id: ctx.guild_id().expect("Guild Only Command"),
                            data: ctx.data().clone(),
                            songbird: manager,
                        }
                    );
                "JOIN!".to_owned()
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
#[command(prefix_command, slash_command, guild_only, aliases("l"))]
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
#[command(prefix_command, slash_command, guild_only, aliases("p"))]
pub async fn play(
    ctx: Context<'_>,
    #[description = "The Youtube link you want to play"]
    #[description_localized("zh-TW", "想要播放的Youtube連結")]
    url: String,
) -> anyhow::Result<()> {
    let guild_id = ctx.guild_id().expect("Guild Only Command");
    let parse_result = AudioLink::parse(&url).await;
    let mut map = ctx.data().song_queue.lock().await;
    let state = map.entry(guild_id).or_insert_with(QueueState::new);
    match parse_result {
        Ok(ParseResult::Single(audio)) => {
            if state.playing {
                state.queue.push_back(audio);
                ctx.say("Added to queue!").await?;
            } else {
                state.playing = true;
                let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
                let call = manager.get_or_insert(guild_id);
                (*call).lock().await.play(audio.into());
                ctx.say("Play!").await?;
            }
        },
        Ok(ParseResult::Multiple(audio_list, meta)) => {
            let list_len = audio_list.len();
            state.queue.append(&mut audio_list.into());
            if !state.playing {
                if let Some(audio) = state.queue.pop_front() {
                    state.playing = true;
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

/// Stop playing songs
#[command(prefix_command, slash_command, guild_only, aliases("s"))]
pub async fn stop(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let mut map = ctx.data().song_queue.lock().await;
    let state = map.entry(ctx.guild_id().unwrap()).or_insert_with(QueueState::new);
    state.playing = false;
    state.queue.clear();
    let manager = songbird::get(&ctx.serenity_context()).await.expect("Songbird Not initialized");
    let call = manager.get_or_insert(ctx.guild_id().unwrap());
    (*call).lock().await.stop();
    ctx.say("Stop!").await?;
    Ok(())
}

/// List songs in the play queue
#[command(prefix_command, slash_command, guild_only, aliases("q"))]
pub async fn queue(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    let mut map = ctx.data().song_queue.lock().await;
    let state = map.entry(ctx.guild_id().unwrap()).or_insert_with(QueueState::new);
    if state.queue.len() == 0 {
        ctx.say("There's no song in the queue").await?;
        return Ok(());
    }
    let body = state.queue.iter()
        .map(|entry| match entry {
            AudioLink::Youtube(info) => {
                format!("- `{}` [{}:{:02}]", info.title, info.duration / 60, info.duration % 60)
            },
        })
        .fold(format!("Total of {} songs:", state.queue.len()), |acc, e| acc + "\n" + &e);
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
            let mut map = self.data.song_queue.lock().await;
            let state = map.entry(self.guild_id).or_insert_with(QueueState::new);
            if let Some(next_song) = state.queue.pop_front() {
                let call = self.songbird.get_or_insert(self.guild_id);
                (*call).lock().await.play(next_song.into());
            } else {
                state.playing = false;
            }
        }
        None
    }
}
