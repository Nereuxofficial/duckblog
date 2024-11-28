# Adapted from https://kerkour.com/rust-small-docker-image
## Builder
# TODO: Do a scratch image
FROM rust:slim-bookworm AS builder
LABEL authors="Nereuxofficial"

RUN rustup target add x86_64-unknown-linux-musl
RUN update-ca-certificates
RUN apt update && apt upgrade -y
RUN apt install -y musl-tools musl-dev

# Create appuser
ENV USER=duckblog
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

WORKDIR /duckblog

COPY ./content ./content
COPY ./static ./static
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./liquid ./liquid
COPY ./src ./src

RUN cargo b -r --target x86_64-unknown-linux-musl

## Run image
FROM scratch

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /duckblog

# Copy our build
COPY --from=builder /duckblog/target/x86_64-unknown-linux-musl/release/duckblog .

# Use an unprivileged user.
USER duckblog:duckblog

EXPOSE 80

# Run the binary.
ENTRYPOINT ["./duckblog"]
