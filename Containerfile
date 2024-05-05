FROM rust:alpine as builder

RUN apk add musl-dev
RUN cargo new --bin /builder
WORKDIR /builder
COPY ["Cargo.toml", "Cargo.lock", "./"]
RUN cargo build --release && \
    rm -rf ./src

COPY src ./src
RUN rm target/release/deps/website_backend2* && \
    cargo build --release && \
    strip -s target/release/website_backend2




FROM alpine

WORKDIR /app
COPY --from=builder /builder/target/release/website_backend2 ./
EXPOSE 3000

RUN addgroup -S backend2 && \
    adduser -S -D -H -s /bin/false -G backend2 backend2 && \
    chown -R backend2:backend2 /app
USER backend2

CMD ["./website_backend2"]

