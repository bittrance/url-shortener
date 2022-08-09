FROM rust:1.61 as builder
WORKDIR /usr/src/url-shortener
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/url-shortener /usr/local/bin/url-shortener
CMD ["url-shortener"]