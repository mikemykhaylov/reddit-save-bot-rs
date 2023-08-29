use reqwest::{Client, Body, multipart};
use serde::Deserialize;
use tokio::fs::{File, self};
use tokio_util::codec::{FramedRead, BytesCodec};

#[derive(Deserialize, Debug)]
pub struct Update {
    update_id: i64,
    pub message: Message,
}

#[derive(Deserialize, Debug)]
pub struct Message {
    message_id: i64,
    pub from: User,
    date: i64,
    pub text: String,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub id: i64,
    is_bot: bool,
    first_name: String,
    last_name: Option<String>,
    username: Option<String>,
}

pub struct TelegramAPI {
    token: String,
    client: Client,
}

impl TelegramAPI {
    pub fn new(token: String) -> TelegramAPI {
        TelegramAPI {
            token,
            client: Client::new(),
        }
    }

    pub async fn send_message(&self, chat_id: i64, text: String) -> Result<(), reqwest::Error> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage?chat_id={}&text={}",
            self.token, chat_id, text
        );
        let res = &self.client.get(&url).send().await?;
        println!("Status: {}", res.status());
        Ok(())
    }

    pub async fn send_video(&self, chat_id: i64, video: String) -> Result<(), reqwest::Error> {
        let file = fs::read(&video).await.expect("Unable to open file");

        println!("Opened file");

        //make form part of file
        let some_file = multipart::Part::bytes(file)
            .file_name(video)
            .mime_str("video/mp4")?;

        println!("Created part");

        //create the multipart form
        let form = multipart::Form::new()
            .part("video", some_file);

        println!("Created form");

        //send request
        let url = format!(
            "https://api.telegram.org/bot{}/sendVideo?chat_id={}",
            self.token, chat_id
        );
        let res = &self.client.post(url).multipart(form).send().await?;
        println!("Status: {}", res.status());
        Ok(())
    }
}
