use std::process::Stdio;

use serde::Deserialize;

use serde_json::Value;

use tokio::process::Command;

use tracing::instrument;

use urlencoding::encode;

use crate::structs::AudioLink;

pub enum InfoType {
    Video(YoutubeInfo),
    Playlist(Vec<YoutubeInfo>),
}

#[instrument]
pub async fn get_yt_info(url: &str) -> Result<InfoType, Error> {
    let output = Command::new("yt-dlp")
        .arg("-j")
        .arg("--flat-playlist")
        .arg("--skip-download")
        .arg("--no-warning")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?
        .wait_with_output()
        .await?;

    let result = std::str::from_utf8(&output.stdout)?;

    if !output.status.success() {
        return Err(Error::CommandError(result.to_string()));
    }

    let list = result.lines()
        .map(serde_json::from_str::<YoutubeInfo>)
        .flatten()  // ignore the failed items
        .collect::<Vec<_>>();

    if list.len() == 1 {
        Ok(InfoType::Video(list.into_iter().next().unwrap()))
    } else if list.len() > 1 {
        Ok(InfoType::Playlist(list))
    } else {
        Err(Error::UnknownParseError)
    }
}

#[instrument]
pub async fn search_yt(prompt: &str) -> Result<Vec<YoutubeInfo>, Error> {
    let output = Command::new("yt-dlp")
        .arg("-j")
        .arg("--flat-playlist")
        .arg("--skip-download")
        .arg("--no-warning")
        .arg("--match-filter")
        .arg("original_url!*=/shorts/ & url!*=/shorts/")
        .arg("--playlist-items")
        .arg("1:70")
        .arg(format!("https://www.youtube.com/results?sp=CAASAhAB&search_query={}", encode(prompt)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?
        .wait_with_output()
        .await?;

    let result = std::str::from_utf8(&output.stdout)?;

    if !output.status.success() {
        return Err(Error::CommandError(result.to_string()));
    }

    let list = result.lines()
        .flat_map(serde_json::from_str::<Value>)
        .flat_map(|mut v| {
            if let Value::Object(ref mut map) = v {
                let r = map.get_mut("duration")?;
                if let Value::Number(n) = r {
                    if !n.is_u64() {
                        *n = (n.as_f64()? as u64).into();
                    }
                    return Some(v);
                }
            }
            None
        })
        .flat_map(serde_json::from_value::<YoutubeInfo>)
        .collect::<Vec<_>>();

    Ok(list)
}

#[derive(Debug, Deserialize)]
pub struct YoutubeInfo {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub channel: String,
    pub channel_url: String,
    pub duration: u32,
    pub playlist: Option<String>,
}

impl From<YoutubeInfo> for AudioLink {
    fn from(value: YoutubeInfo) -> Self {
        AudioLink::Youtube(value)
    }
}

#[derive(Debug)]
pub enum Error {
    SerdeJsonError(serde_json::Error),
    StdIOError(std::io::Error),
    Utf8Error(std::str::Utf8Error),
    CommandError(String),
    UnknownParseError,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::SerdeJsonError(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::StdIOError(value)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(value: std::str::Utf8Error) -> Self {
        Error::Utf8Error(value)
    }
}
