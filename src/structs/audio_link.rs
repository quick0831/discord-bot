use songbird::input::Input;
use songbird::input::YoutubeDl;

use crate::CLIENT;

pub enum AudioLink {
    Youtube(String),
}

impl AudioLink {
    pub fn parse(link: &str) -> Option<AudioLink> {
        Some(Self::Youtube(link.to_string()))
    }
}

// Some test links
// https://www.youtube.com/watch?v=t9CKSSG96DU
// https://youtu.be/t9CKSSG96DU?si=PlP9JU3JO-z_fbtN
// https://music.youtube.com/watch?v=yfrBPWDTCpE&list=RDAMVMyfrBPWDTCpE
// https://music.youtube.com/watch?v=yfrBPWDTCpE&si=srWTv-Qi7Rk3PvLA
// https://music.youtube.com/playlist?list=OLAK5uy_nS1xNEN2T0cy8CU4LpgPYhs7qKWrCTykI&si=XLw3liCitNXrkVnl
// https://music.youtube.com/playlist?list=OLAK5uy_nS1xNEN2T0cy8CU4LpgPYhs7qKWrCTykI

impl From<AudioLink> for Input {
    fn from(audio: AudioLink) -> Self {
        let client = CLIENT.get().expect("Client Initialized").clone();
        match audio {
            AudioLink::Youtube(url) => YoutubeDl::new(client, url).into(),
        }
    }
}
