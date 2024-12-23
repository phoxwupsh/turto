FROM rust:1.81-alpine AS builder
WORKDIR /build
# it seems openssl does not work thus switch to libressl
RUN apk update && apk add git make cmake musl-dev libressl-dev

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM alpine:3.18
WORKDIR /app

RUN apk add --no-cache python3 xz

# download yt-dlp
RUN wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp

# copy bot binary
COPY --from=builder /build/target/release/turto .

# copy config files
COPY config.toml.template ./config.toml
COPY help.toml.template ./help.toml
COPY templates.toml.template ./templates.toml
COPY .env.template ./.env

ENTRYPOINT ["/app/turto"]