FROM rustlang/rust:nightly as builder

RUN USER=root cargo new --bin keeper_extractor
WORKDIR /keeper_extractor

# copy over your manifests
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
RUN rm ./target/release/deps/keeper_extractor*
COPY ./src ./src

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

COPY --from=builder /keeper_extractor/target/release/keeper_extractor /keeper_extractor/keeper_extractor
WORKDIR /keeper_extractor/


CMD ["/keeper_extractor/keeper_extractor"]
