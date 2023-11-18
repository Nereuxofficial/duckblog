# Adapted from https://kerkour.com/rust-small-docker-image
## Builder
FROM rust:latest AS builder
LABEL authors="Nereuxofficial"

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates


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

COPY ./ .

RUN cargo build --release --target x86_64-unknown-linux-musl

## Run image
FROM scratch

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /duckblog

# Copy our build
COPY --from=builder /duckblog/target/x86_64-unknown-linux-musl/release/duckblog .

# Use an unprivileged user.
USER duckblog:duckblog

# Run the binary.
ENTRYPOINT ["./duckblog"]