FROM node:16-buster-slim AS node-builder
COPY . .
WORKDIR /ctx/client
RUN npm ci
RUN npm run lint
RUN npm run build
RUN npm run export
RUN rm -rf out/runtime-config.json

FROM rust:slim-buster AS rust-builder
RUN apt update
RUN apt install pkg-config libssl-dev -y
COPY . .
WORKDIR /ctx/server
RUN OPENSSL_STATIC=1 OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include/openssl \
  cargo build -r

FROM debian:buster-slim
RUN apt-get update && \
  apt-get --no-install-recommends -y install ca-certificates && \
  apt-get clean && \
  rm -rf /var/lib/apt/lists/*
WORKDIR /ywinby
COPY --from=rust-builder /ctx/server/target/release/ywinby .
COPY --from=node-builder /ctx/client/out ./static
RUN useradd ywinby -M -u 1000 -d /ywinby
RUN mkdir -p db && chown root:ywinby db static && chmod 770 db static
USER ywinby
EXPOSE 8080
VOLUME [ "/ywinby/db" ]
CMD ["./ywinby"]
