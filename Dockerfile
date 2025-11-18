FROM rust:1.91.1-slim AS build
RUN rustup target add x86_64-unknown-linux-musl && \
    apt update && \
    apt install -y musl-tools musl-dev && \
    update-ca-certificates

COPY ./ ./

RUN cargo build --profile=release --target=x86_64-unknown-linux-musl

FROM rust:1.91.1-alpine3.22

COPY --from=build ./target/x86_64-unknown-linux-musl/release/chef /app/chef

EXPOSE 9080
WORKDIR "/recipes"
ENTRYPOINT ["/app/chef", "serve", "--host", "--port=9080"]