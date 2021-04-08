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

COPY --from=builder /extractor/target/release/ftm_pulse_catcher /ftm_pulse_catcher/ftm_pulse_catcher
WORKDIR /ftm_pulse_catcher
EXPOSE 8000


CMD ["/ftm_pulse_catcher/ftm_pulse_catcher"]
