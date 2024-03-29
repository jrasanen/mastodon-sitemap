FROM rust:1.77 as builder
WORKDIR /usr/src/app
COPY . .
RUN RUSTFLAGS="-C link-arg=-s" cargo build --release

FROM debian:buster-slim
COPY --from=builder /usr/src/app/target/release/mastodon-sitemap /usr/local/bin/mastodon-sitemap
CMD ["mastodon-sitemap"]