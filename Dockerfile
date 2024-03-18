FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y yt-dlp ffmpeg && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/reddit-save-bot /usr/local/bin/
ENTRYPOINT ["/usr/local/bin/reddit-save-bot"]
