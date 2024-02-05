use std::process::Stdio;

use serde::Deserialize;

use serde_json::Value;

use tokio::process::Command;

pub enum InfoType {
    Video(YoutubeInfo),
    Playlist(YoutubePlaylistInfo),
}

pub async fn get_yt_info(url: &str) -> Result<InfoType, Error> {
    let output = Command::new("yt-dlp")
        .arg("-J")
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

    let json: Value = serde_json::from_str(&result)?;

    if let Some(Value::String(t)) = json.get("_type") {
        if t == "video" {
            let info: YoutubeInfo = serde_json::from_value(json)?;
            return Ok(InfoType::Video(info));
        }
        if t == "playlist" {
            let info: YoutubePlaylistInfo = serde_json::from_value(json)?;
            return Ok(InfoType::Playlist(info));
        }
    }
    Err(Error::UnknownParseError)
}


#[derive(Deserialize)]
pub struct YoutubeInfo {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub channel: String,
    pub channel_url: String,
    pub duration: u32,
}

#[derive(Deserialize)]
pub struct YoutubePlaylistInfo {
    pub title: String,
    pub entries: Vec<YoutubeInfo>,
}

pub enum Error {
    SerdeJsonError(serde_json::Error),
    StdIOError(std::io::Error),
    Utf8Error(std::str::Utf8Error),
    CommandError(String),
    UnknownParseError,
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
