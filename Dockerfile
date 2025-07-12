FROM rust:1.86

RUN cargo install cargo-watch
WORKDIR /app
COPY . .
RUN cargo fetch

CMD ["cargo", "watch", "-x", "run"]
