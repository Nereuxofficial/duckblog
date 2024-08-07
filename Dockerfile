# Adapted from https://kerkour.com/rust-small-docker-image
## Builder
# TODO: Do a scratch image
FROM rust:slim-bookworm AS builder
LABEL authors="Nereuxofficial"

RUN update-ca-certificates
RUN apt update && apt upgrade -y

# Create appuser
ENV USER=duckblog
ENV UID=10001
ENV RUST_BACKTRACE=1


RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

WORKDIR /duckblog

COPY ./ .

RUN cargo b -r

## Run image
FROM debian:bookworm-slim

RUN apt update && apt upgrade -y
RUN apt install -y openssl pkg-config ca-certificates libssl-dev curl
RUN update-ca-certificates

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /duckblog

# Copy our build
COPY --from=builder /duckblog/target/release/duckblog .

# Use an unprivileged user.
USER duckblog:duckblog

EXPOSE 80

# Run the binary.
ENTRYPOINT ["./duckblog"]
