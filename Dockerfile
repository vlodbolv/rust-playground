FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY server/ ./
RUN cargo build --release

FROM alpine:latest

RUN apk add --no-cache \
    rust \
    cargo \
    musl-dev \
    gcc

COPY --from=builder /app/target/release/rust-playground /usr/local/bin/

COPY frontend/ /app/frontend/

WORKDIR /app

EXPOSE 8080

CMD ["rust-playground"]
