FROM rustlang/rust:nightly as builder

ARG name

COPY . .
#RUN USER=root cargo new --bin $name
WORKDIR /${name}

# copy over your manifests
#COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
#RUN rm src/*.rs

# copy your source tree
#RUN rm ./target/release/deps/$name*
#COPY ./src ./src

#RUN cargo build --release

FROM debian:buster-slim
ARG name

RUN apt-get update && \
    apt-get --assume-yes install \
        make \
        libpq5 \
        libpq-dev \
        -qqy \
        --no-install-recommends
RUN apt-get update && apt-get -y install ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

COPY --from=builder /${name}/target/release/${name} /${name}/${name}
WORKDIR /${name}/


CMD ["/${name}/${name}"]
