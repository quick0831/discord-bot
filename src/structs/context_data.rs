use std::collections::VecDeque;
use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;

use serenity::all::GuildId;
use serenity::all::UserId;

use super::AudioLink;

#[derive(Debug)]
pub struct PerGuildData {
    pub player: PlayerData,
}

#[derive(Debug)]
pub struct PlayerData {
   pub queue: VecDeque<AudioLink>,
   pub state: PlayerState,
   pub loop_policy: LoopPolicy,
   pub search_item: HashMap<UserId, Vec<AudioLink>>
}

#[derive(Debug)]
pub enum PlayerState {
    Offline,
    Idle,
    Playing(AudioLink),
}

/// Determine what to do after the song ends
#[derive(Debug)]
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
                search_item: HashMap::new(),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Data(Arc<DashMap<GuildId, PerGuildData>>);

impl Data {
    pub fn new() -> Self {
        Data(Arc::new(DashMap::new()))
    }
    
    pub fn get(&self, guild_id: GuildId) -> dashmap::mapref::one::RefMut<'_, GuildId, PerGuildData> {
        self.0.entry(guild_id).or_insert_with(PerGuildData::new)
    }
}
