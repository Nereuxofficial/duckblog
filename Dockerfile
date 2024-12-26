# Adapted from https://kerkour.com/rust-small-docker-image
## Builder
FROM rust:alpine AS builder
LABEL authors="Nereuxofficial"

RUN apk add musl-dev

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

COPY ./static ./static
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./liquid ./liquid
COPY ./src ./src
COPY ./content ./content

RUN cargo b -r

## Run image
FROM scratch

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
