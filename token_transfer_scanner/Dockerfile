FROM rust:1.50 as builder

WORKDIR /extractor
COPY . .
RUN cargo clean
RUN cargo build --release

FROM debian:buster-slim

RUN apt-get update && \
    apt-get --assume-yes install \
        make \
        libpq5 \
        libpq-dev \
        -qqy \
        --no-install-recommends
RUN apt-get update && apt-get -y install ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

COPY --from=builder /extractor/target/release/token_transfer_scanner /extractor_core/token_transfer_scanner
WORKDIR /extractor_core
EXPOSE 8000


CMD ["/extractor_core/token_transfer_scanner"]
