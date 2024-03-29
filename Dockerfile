FROM rust:1.77

WORKDIR /usr/src/app

COPY . .

RUN cargo install --release --path .

CMD ["mastodon-sitemap"]
