FROM rust:alpine AS builder
WORKDIR /build

RUN apk update && apk add git cmake make musl-dev

COPY Cargo.toml Cargo.lock ./
COPY turto-macros ./turto-macros
COPY src ./src
RUN cargo build --release

FROM alpine:latest
WORKDIR /app

RUN apk add --no-cache ca-certificates libstdc++ libgcc

# copy bot binary
COPY --from=builder /build/target/release/turto .

ENTRYPOINT ["/app/turto"]