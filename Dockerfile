FROM rust:alpine3.22 AS builder
WORKDIR /build

RUN apk update && apk add git make cmake musl-dev openssl-dev openssl-libs-static

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM alpine:3.22
WORKDIR /app

RUN apk add --no-cache ca-certificates libstdc++ libgcc

# copy bot binary
COPY --from=builder /build/target/release/turto .

ENTRYPOINT ["/app/turto"]