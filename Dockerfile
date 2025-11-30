FROM rust:1.91.1-slim AS build
RUN rustup target add x86_64-unknown-linux-musl && \
    apt-get update && \
    apt-get install -y musl-tools musl-dev && \
    update-ca-certificates

COPY ./ ./

RUN cargo build --profile=release --target=x86_64-unknown-linux-musl --locked

FROM rust:1.91.1-alpine3.22

COPY --from=build ./target/x86_64-unknown-linux-musl/release/chef /app/chef

ENV PUID=1001 PGID=1001
RUN addgroup -g ${PGID} chef_user
RUN adduser -u ${PUID} -G chef_user -s /bin/sh -D chef_user
USER ${PUID}:${PGID}

EXPOSE 9080
WORKDIR "/recipes"
ENTRYPOINT ["/app/chef", "serve", "--host", "--port=9080", "--disable-open-editor"]
