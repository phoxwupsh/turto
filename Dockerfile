FROM rust:1.73-alpine as builder
WORKDIR /build
COPY . .
RUN apk update && apk add git make cmake musl-dev
RUN cargo build --release

FROM alpine:3.18
WORKDIR /app

RUN apk add --no-cache python3 xz

# download yt-dlp
RUN wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp

# copy bot binary
COPY --from=builder /build/target/release/turto .
COPY --from=builder /build/config.toml.template ./config.toml
COPY --from=builder /build/help.toml.template ./help.toml
COPY --from=builder /build/templates.toml.template ./templates.toml

ENTRYPOINT ["/app/turto"]