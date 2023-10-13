FROM rust:1.73-bullseye as builder
WORKDIR /build
COPY . .
RUN apt-get update && apt-get install -y cmake
RUN cargo build --release

FROM debian:bullseye-slim
WORKDIR /app

RUN apt-get update && apt-get install -y xz-utils wget

# download yt-dlp
RUN wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux -O /usr/local/bin/yt-dlp
RUN chmod a+rx /usr/local/bin/yt-dlp

# install static ffmpeg
RUN wget https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz \
    && tar -xJf ffmpeg-release-amd64-static.tar.xz -C /usr/local/ \
    && rm ffmpeg-release-amd64-static.tar.xz \
    && FFMPEG_DIR=$(ls /usr/local | grep -o "ffmpeg-[0-9]\+.*") \
    && mv /usr/local/$FFMPEG_DIR /usr/local/ffmpeg \
    && mv /usr/local/ffmpeg/ffmpeg /usr/local/bin/ffmpeg \
    && chmod a+rx /usr/local/bin/ffmpeg \
    && rm -r /usr/local/ffmpeg

# copy bot binary
COPY --from=builder /build/target/release/turto .
COPY --from=builder /build/config.toml.template ./config.toml
COPY --from=builder /build/help.toml.template ./help.toml
COPY --from=builder /build/templates.toml.template ./templates.toml

CMD ["/app/turto"]