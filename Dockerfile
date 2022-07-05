FROM rust:alpine as builder

RUN apk add musl-dev
WORKDIR /builder
RUN cargo new --bin app
WORKDIR /builder/app
COPY ["Cargo.toml", "Cargo.lock", "./"]
RUN cargo build --release && \
    rm -rf ./src

COPY src ./src
RUN rm target/release/deps/website_backend2* && \
    cargo build --release && \
    strip -s target/release/website_backend2

FROM alpine
WORKDIR /app
COPY --from=builder /builder/app/target/release/website_backend2 ./
EXPOSE 3000
CMD ["./website_backend2"]

