FROM rust:alpine AS builder
WORKDIR /build
# it seems openssl does not work thus switch to libressl
RUN apk update && apk add git make cmake musl-dev openssl-dev openssl-libs-static

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM alpine:latest
WORKDIR /app

RUN apk add --no-cache ca-certificates libstdc++ libgcc

# copy bot binary
COPY --from=builder /build/target/release/turto .

# copy config files
COPY config.toml.template ./config.toml
COPY help.toml.template ./help.toml
COPY templates.toml.template ./templates.toml
COPY .env.template ./.env

# download latest yt-dlp when container start
ENTRYPOINT ["/app/turto"]