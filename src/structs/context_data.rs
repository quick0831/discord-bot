use std::collections::VecDeque;
use std::sync::Arc;

use dashmap::DashMap;

use serenity::all::GuildId;

use super::AudioLink;

pub struct PerGuildData {
    pub player: PlayerData,
}

pub struct PlayerData {
   pub queue: VecDeque<AudioLink>,
   pub state: PlayerState,
   pub loop_policy: LoopPolicy,
}

pub enum PlayerState {
    Offline,
    Idle,
    Playing(AudioLink),
}

/// Determine what to do after the song ends
pub enum LoopPolicy {
    /// Drop the song after it ends
    Normal,
    /// Add the song back to the queue
    Loop,
    /// Put the song in the shuffle pool
    Random,
}

impl PerGuildData {
    pub fn new() -> Self {
        PerGuildData {
            player: PlayerData {
                queue: VecDeque::new(),
                state: PlayerState::Offline,
                loop_policy: LoopPolicy::Normal,
            }
        }
    }
}

pub struct Data(Arc<DashMap<GuildId, PerGuildData>>);

impl Data {
    pub fn new() -> Self {
        Data(Arc::new(DashMap::new()))
    }
    
    pub fn get(&self, guild_id: GuildId) -> dashmap::mapref::one::RefMut<'_, GuildId, PerGuildData> {
        self.0.entry(guild_id).or_insert_with(PerGuildData::new)
    }
}

impl Clone for Data {
    fn clone(&self) -> Self {
        Data(self.0.clone())
    }
}
