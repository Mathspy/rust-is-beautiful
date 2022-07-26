FROM rust:1.62 as builder
WORKDIR /usr/src/rust-is-beautiful
COPY . .
RUN cargo install --path .
FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/rust-is-beautiful /usr/local/bin/rust-is-beautiful

ARG GITHUB_TOKEN
ENV GITHUB_TOKEN=$GITHUB_TOKEN
ARG MAGIC_NUMBER
ENV MAGIC_NUMBER=$MAGIC_NUMBER

COPY ./assets/ assets

CMD rust-is-beautiful
