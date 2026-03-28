# Build stage
FROM rustlang/rust:nightly-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig cmake make
ENV OPENSSL_NO_VENDOR=1
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
