FROM node:16-buster-slim AS node-builder
COPY . .
WORKDIR /ctx/client
RUN npm ci
RUN npm run lint
RUN npm run build
RUN npm run export
RUN rm -rf out/runtime-config.json

FROM rust:slim as rust-builder
RUN apt update
RUN apt install pkg-config libssl-dev -y
RUN rustup toolchain install nightly
RUN rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
COPY . .
WORKDIR /ctx/server
RUN RUSTFLAGS="-C target-feature=+crt-static -Zlocation-detail=none" \
    OPENSSL_STATIC=1 OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include/openssl \
    cargo +nightly build -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --target x86_64-unknown-linux-gnu --release
RUN rm -rf target/release && mv target/x86_64-unknown-linux-gnu/release target/release

FROM alpine
WORKDIR /ywinby
COPY --from=rust-builder /ctx/server/target/release/ywinby .
COPY --from=node-builder /ctx/client/out ./static
RUN adduser -h /ywinby -H -u 1000 -D ywinby
RUN mkdir -p db && chown root:ywinby db static && chmod 770 db static
USER ywinby
EXPOSE 8080
VOLUME [ "/ywinby/db" ]
CMD ["./ywinby"]
