FROM rust:1.81-alpine AS builder
WORKDIR /build
# it seems openssl does not work thus switch to libressl
RUN apk update && apk add git make cmake musl-dev libressl-dev

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM alpine:3.18
WORKDIR /app

RUN apk add --no-cache python3 ca-certificates dcron

# setup yt-dlp update cron job
RUN mkdir -p /etc/periodic/daily/
COPY update-yt-dlp.sh /etc/periodic/daily/update-yt-dlp
RUN chmod a+rx /etc/periodic/daily/update-yt-dlp

# copy bot binary
COPY --from=builder /build/target/release/turto .

# copy config files
COPY config.toml.template ./config.toml
COPY help.toml.template ./help.toml
COPY templates.toml.template ./templates.toml
COPY .env.template ./.env

# download latest yt-dlp when container start
ENTRYPOINT ["sh", "-c", "/etc/periodic/daily/update-yt-dlp && crond -b -l 1 && /app/turto"]