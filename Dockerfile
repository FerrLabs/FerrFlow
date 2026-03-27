# Build stage
FROM rustlang/rust:nightly-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev pkgconfig cmake make perl
WORKDIR /app
# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -f target/release/ferrflow target/release/deps/ferrflow*
# Build for real
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM alpine:3.23
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/ferrflow /usr/local/bin/ferrflow
ENTRYPOINT ["ferrflow"]
CMD ["--help"]
