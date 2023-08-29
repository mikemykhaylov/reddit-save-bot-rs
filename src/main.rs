mod api;

use std::{env, path::PathBuf};

use api::telegram::{TelegramAPI, Update};
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use url::Url;
use uuid::Uuid;
use ytd_rs::{Arg, YoutubeDL};

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new()
        .route("/", get(handler))
        .route("/webhook", post(get_video));

    // get PORT from env variable
    let port = std::env::var("PORT").unwrap().parse::<u16>().unwrap();
    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

async fn get_video(Json(request): Json<serde_json::Value>) -> impl IntoResponse {
    // load environment variables
    let token = env::var("TELEGRAM_BOT_TOKEN").unwrap();
    let personal_id = env::var("PERSONAL_ID").unwrap();

    // create a new TelegramAPI instance
    let api = TelegramAPI::new(token);

    // try to deserialize the value into our struct
    let update: Update = serde_json::from_value(request).expect("Request should be a valid Update");

    // check if the message is from the personal id
    // don't notify if it's not, as this prevents spam
    if update.message.from.id.to_string() != personal_id {
        return (StatusCode::OK, "Ok");
    }

    // if it's a /start command, send a message welcoming the user
    if update.message.text == "/start" {
        api.send_message(update.message.from.id, "Hello!".to_string())
            .await
            .unwrap();
    }

    // try parsing the message text as a URL
    let url = match Url::parse(&update.message.text) {
        Ok(url) => url,
        Err(_) => {
            api.send_message(
                update.message.from.id,
                "Please send a valid URL".to_string(),
            )
            .await
            .expect("Failed to send message about invalid URL");
            return (StatusCode::OK, "Ok");
        }
    };

    // check if the URL is a reddit post
    if !url.host_str().unwrap().contains("reddit.com") {
        api.send_message(
            update.message.from.id,
            "Please send a valid reddit post".to_string(),
        )
        .await
        .expect("Failed to send message about invalid reddit post");
        return (StatusCode::OK, "Ok");
    }

    let video_name = Uuid::new_v4().to_string();
    let args = vec![Arg::new_with_arg(
        "--output",
        &format!("{}.%(ext)s", video_name),
    )];
    let path = PathBuf::from("/tmp");
    let ytd =
        YoutubeDL::new(&path, args, url.as_str()).expect("Failed to create YoutubeDL instance");

    // start download
    let download = match ytd.download() {
        Ok(download) => download,
        Err(_) => {
            api.send_message(
                update.message.from.id,
                "Failed to download video".to_string(),
            )
            .await
            .expect("Failed to send message about failed download");
            return (StatusCode::OK, "Ok");
        }
    };

    println!("download: {:?}", download);

    // list files in /tmp
    let files = tokio::fs::read_dir("/tmp")
        .await
        .expect("Failed to read /tmp directory");
    println!("files: {:?}", files);

    api.send_video(update.message.from.id, format!("/tmp/{}.mp4", video_name))
        .await
        .expect("Failed to send video");

    (StatusCode::OK, "Ok")
}
