use std::fmt::Display;

use serde::Deserialize;
use serde::Serialize;
use songbird::input::Input;
use songbird::input::YoutubeDl;

use crate::CLIENT;
use crate::sources::youtube;
use crate::sources::youtube::YoutubeInfo;
use crate::sources::youtube::get_yt_info;

#[derive(Debug, Clone)]
pub enum AudioLink {
    Youtube(YoutubeInfo),
}

/// Lazy version of `AudioLink`, use `load()` to get `AudioLink`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnloadedAudioLink {
    #[serde(rename = "yt")]
    Youtube(String),
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
                Ok(youtube::InfoType::Video(info)) => Ok(ParseResult::Single(AudioLink::Youtube(info))),
                Ok(youtube::InfoType::Playlist(infos)) => {
                    let title = infos[0].playlist.clone().unwrap_or_else(|| String::from("Unknown"));
                    let list = infos.into_iter()
                        .map(|entry| AudioLink::Youtube(entry))
                        .collect();
                    Ok(ParseResult::Multiple(list, Metadata { title }))
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
        match audio {
            AudioLink::Youtube(info) => YoutubeDl::new((*CLIENT).clone(), format!("https://www.youtube.com/watch?v={}", info.id)).into(),
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
            Self::Youtube(info) => info.duration,
        }
    }

    pub fn time_str(&self) -> String {
        let t = self.time();
        format!("{}:{:02}", t / 60, t % 60)
    }

    pub fn unload(&self) -> UnloadedAudioLink {
        match self {
            Self::Youtube(info) => UnloadedAudioLink::Youtube(info.id.to_owned()),
        }
    }
}

impl UnloadedAudioLink {
    pub async fn load(self) -> anyhow::Result<AudioLink> {
        match self {
            Self::Youtube(id) => Ok(AudioLink::Youtube(youtube::load(&id).await?)),
        }
    }
}
