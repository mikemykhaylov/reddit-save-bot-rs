mod api;
mod logging;

use std::{env, path::PathBuf};

use api::telegram::{TelegramAPI, Update};
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use tokio::fs::File;
use url::Url;
use uuid::Uuid;
use ytd_rs::{Arg, YoutubeDL};

#[tokio::main]
async fn main() {
    // set up logging
    logging::set_up_logger();

    // build our application with a route
    let app = Router::new()
        .route("/", get(handler))
        .route("/webhook", post(get_video));

    // get PORT from env variable
    let port = env::var("PORT").unwrap().parse::<u16>().unwrap();
    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

async fn get_video(Json(request): Json<serde_json::Value>) -> impl IntoResponse {
    let operation_id = &Uuid::new_v4().to_string();

    log::info!(target: operation_id, "Started handler");

    // load environment variables
    let token = env::var("TELEGRAM_BOT_TOKEN").unwrap();
    let personal_id = env::var("PERSONAL_ID").unwrap();

    // create a new TelegramAPI instance
    let api = TelegramAPI::new(token);

    // try to deserialize the value into our struct
    let update: Update = match serde_json::from_value(request.clone()) {
        Ok(update) => update,
        Err(_) => {
            // if it fails, be silent to prevent spam
            log::error!(target: operation_id, "Failed to deserialize update: {}", request);
            return (StatusCode::OK, "Ok");
        }
    };

    // check if the message is from the personal id
    // don't notify if it's not, as this prevents spam
    if update.message.from.id.to_string() != personal_id {
        log::info!(target: operation_id,
            "Message is not from personal id: {}",
            update.message.from.id
        );
        return (StatusCode::OK, "Ok");
    }

    // if it's a /start command, send a message welcoming the user
    if update.message.text == "/start" {
        if let Err(e) = api
            .send_message(update.message.from.id, "Hello!".to_string())
            .await
        {
            log::error!(target: operation_id, "Failed to send message: {}", e);
        }
        return (StatusCode::OK, "Ok");
    }

    // try parsing the message text as a URL
    let url = match Url::parse(&update.message.text) {
        Ok(url) => url,
        Err(_) => {
            log::warn!(target: operation_id, "Failed to parse URL: {}", update.message.text);
            if let Err(e) = api
                .send_message(
                    update.message.from.id,
                    "Please send a valid URL".to_string(),
                )
                .await
            {
                log::error!(target: operation_id, "Failed to send message: {}", e);
            }
            return (StatusCode::OK, "Ok");
        }
    };

    // check if the URL is a reddit post
    if !url.host_str().unwrap().contains("reddit.com") {
        log::warn!(target: operation_id, "URL is not a reddit post: {}", update.message.text);
        if let Err(e) = api
            .send_message(
                update.message.from.id,
                "Please send a valid reddit post".to_string(),
            )
            .await
        {
            log::error!(target: operation_id, "Failed to send message: {}", e);
        }
        return (StatusCode::OK, "Ok");
    }

    let video_name = Uuid::new_v4().to_string();
    let args = vec![Arg::new_with_arg(
        "--output",
        &format!("{}.%(ext)s", video_name),
    )];
    let path = PathBuf::from("/tmp");
    let ytd = match YoutubeDL::new(&path, args, url.as_str()) {
        Ok(ytd) => ytd,
        Err(_) => {
            log::error!(target: operation_id, "Failed to create YoutubeDL instance");
            if let Err(e) = api
                .send_message(
                    update.message.from.id,
                    "Failed to download video".to_string(),
                )
                .await
            {
                log::error!(target: operation_id, "Failed to send message: {}", e);
            }
            return (StatusCode::OK, "Ok");
        }
    };

    // start download
    match ytd.download() {
        Ok(download) => download,
        Err(_) => {
            log::error!(target: operation_id, "Failed to start download");
            if let Err(e) = api
                .send_message(
                    update.message.from.id,
                    "Failed to download video".to_string(),
                )
                .await
            {
                log::error!(target: operation_id, "Failed to send message: {}", e);
            }
            return (StatusCode::OK, "Ok");
        }
    };

    // list files in /tmp
    let mut files = match tokio::fs::read_dir("/tmp").await {
        Ok(files) => files,
        Err(_) => {
            log::error!(target: operation_id, "Failed to read /tmp");
            if let Err(e) = api
                .send_message(
                    update.message.from.id,
                    "Failed to download video".to_string(),
                )
                .await
            {
                log::error!(target: operation_id, "Failed to send message: {}", e);
            }
            return (StatusCode::OK, "Ok");
        }
    };

    match files.next_entry().await.unwrap() {
        Some(video) => {
            log::info!(target: operation_id, "Video file: {:?}", video.path());
        }
        None => {
            log::error!(target: operation_id, "Video not found");
            return (StatusCode::OK, "Ok");
        }
    };

    // check if the video is larger than 50MB
    // if it is, send a message saying that the video is too large
    // and delete the video
    let file = match File::open(format!("/tmp/{}.mp4", video_name)).await {
        Ok(file) => file,
        Err(_) => {
            log::error!(target: operation_id, "Failed to open video");
            if let Err(e) = api
                .send_message(
                    update.message.from.id,
                    "Failed to download video".to_string(),
                )
                .await
            {
                log::error!(target: operation_id, "Failed to send message: {}", e);
            }
            return (StatusCode::OK, "Ok");
        }
    };

    let metadata = match file.metadata().await {
        Ok(metadata) => metadata,
        Err(_) => {
            log::error!(target: operation_id, "Failed to get video metadata");
            if let Err(e) = api
                .send_message(
                    update.message.from.id,
                    "Failed to download video".to_string(),
                )
                .await
            {
                log::error!(target: operation_id, "Failed to send message: {}", e);
            }
            return (StatusCode::OK, "Ok");
        }
    };

    let size = metadata.len();
    if size > 50 * 1024 * 1024 {
        log::warn!(target: operation_id, "Video is too large to send: {} bytes", size);
        if let Err(e) = api
            .send_message(update.message.from.id, "Video is too large".to_string())
            .await
        {
            log::error!(target: operation_id, "Failed to send message: {}", e);
        }
        if let Err(e) = tokio::fs::remove_file(format!("/tmp/{}.mp4", video_name)).await {
            log::error!(target: operation_id, "Failed to delete video: {}", e);
        }
        return (StatusCode::OK, "Ok");
    }

    match api
        .send_video(update.message.from.id, format!("/tmp/{}.mp4", video_name))
        .await
    {
        Ok(_) => {
            log::info!(target: operation_id, "Video sent");
        }
        Err(e) => {
            log::error!(target: operation_id, "Failed to send video: {}", e);
        }
    }

    (StatusCode::OK, "Ok")
}
