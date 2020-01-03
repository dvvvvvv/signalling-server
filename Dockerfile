FROM rust:1.40 as build-stage

RUN apt-get update && apt-get install -y --no-install-recommends cmake musl-tools
RUN rustup target add x86_64-unknown-linux-musl

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN mkdir src/
RUN echo "fn main() {println!(\"empty rust main\")}" > src/main.rs

RUN cargo build --release --target x86_64-unknown-linux-musl
RUN rm src/*.rs
RUN rm target/x86_64-unknown-linux-musl/release/deps/signalling_server*

COPY ./src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl && \
    strip target/x86_64-unknown-linux-musl/release/signalling-server


FROM alpine:latest

COPY --from=build-stage ./target/x86_64-unknown-linux-musl/release/signalling-server /bin/signalling-server

ENTRYPOINT signalling-server
