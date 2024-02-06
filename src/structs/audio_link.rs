use std::fmt::Display;

use songbird::input::Input;
use songbird::input::YoutubeDl;

use crate::CLIENT;
use crate::sources::youtube::InfoType;
use crate::sources::youtube::YoutubeInfo;
use crate::sources::youtube::get_yt_info;

pub enum AudioLink {
    Youtube(YoutubeInfo),
}

pub enum ParseResult {
    Single(AudioLink),
    Multiple(Vec<AudioLink>, Metadata),
}

pub struct Metadata {
    pub title: String,
}

impl AudioLink {
    pub async fn parse(link: impl Into<String>) -> Result<ParseResult, ()> {
        let link = link.into();
        if true {
            match get_yt_info(&link).await {
                Ok(InfoType::Video(info)) => Ok(ParseResult::Single(AudioLink::Youtube(info))),
                Ok(InfoType::Playlist(info)) => {
                    let list = info.entries.into_iter()
                        .map(|entry| AudioLink::Youtube(entry))
                        .collect();
                    Ok(ParseResult::Multiple(list, Metadata { title: info.title }))
                },
                _ => Err(()),
            }
        } else {
            Err(())
        }
    }
}

impl From<AudioLink> for Input {
    fn from(audio: AudioLink) -> Self {
        let client = CLIENT.get().expect("Client Initialized").clone();
        match audio {
            AudioLink::Youtube(info) => YoutubeDl::new(client, format!("https://www.youtube.com/watch?v={}", info.id)).into(),
        }
    }
}

impl Display for AudioLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioLink::Youtube(info) => {
                write!(f, "{}", info.title)
            },
        }
    }
}

impl AudioLink {
    pub fn time(&self) -> u32 {
        match self {
            AudioLink::Youtube(info) => info.duration,
        }
    }

    pub fn time_str(&self) -> String {
        let t = self.time();
        format!("{}:{:02}", t / 60, t % 60)
    }
}
