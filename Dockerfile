FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef
RUN apk add --no-cache openssl-dev
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM alpine:latest AS runtime
RUN apk add --no-cache yt-dlp ffmpeg
RUN addgroup -S myuser && adduser -S myuser -G myuser
COPY --from=builder /app/target/release/reddit-save-bot /usr/local/bin/
USER myuser
CMD ["/usr/local/bin/reddit-save-bot"]
