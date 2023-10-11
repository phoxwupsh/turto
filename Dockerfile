FROM rust:1.73 as builder
WORKDIR /build
COPY . .
RUN apt-get update && apt-get install -y cmake
RUN cargo build --release

FROM ubuntu:22.04
WORKDIR /app

RUN apt-get update && apt-get install -y ffmpeg wget

# download yt-dlp
RUN wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -O /usr/local/bin/yt-dlp
RUN chmod a+rx /usr/local/bin/yt-dlp

# copy bot binary
COPY --from=builder /build/target/release/turto .

CMD ["/app/turto"]