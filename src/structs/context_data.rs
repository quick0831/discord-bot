use std::collections::VecDeque;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use serenity::all::GuildId;

use tokio::sync::Mutex;

use super::AudioLink;

pub struct QueueState {
   pub queue: VecDeque<AudioLink>,
   pub playing: bool
}

impl QueueState {
    pub fn new() -> Self {
        QueueState {
            queue: VecDeque::new(),
            playing: false,
        }
    }
}

pub struct Data(Arc<_Data>);

pub struct _Data {
    pub song_queue: Mutex<HashMap<GuildId, QueueState>>,
}

impl Data {
    pub fn new() -> Self {
        Data(Arc::new(_Data {
            song_queue: Mutex::new(HashMap::new()),
        }))
    }
}

impl Clone for Data {
    fn clone(&self) -> Self {
        Data(self.0.clone())
    }
}

impl Deref for Data {
    type Target = Arc<_Data>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
