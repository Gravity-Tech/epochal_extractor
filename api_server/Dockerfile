FROM rustlang/rust:nightly-buster as builder

RUN USER=root cargo new --bin api_server
WORKDIR /api_server

# copy over your manifests
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src
#COPY ./contracts ./contracts
#COPY . .

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

COPY --from=builder /api_server/target/release/api_server /api_server/api_server
WORKDIR /api_server/
EXPOSE 8088

CMD ["/api_server/api_server"]
