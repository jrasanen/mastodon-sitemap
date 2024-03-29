FROM rust:1.77

WORKDIR /usr/src/app

COPY . .

RUN cargo install --path .

CMD ["mastodon-sitemap"]
