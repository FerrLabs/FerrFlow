# Build stage
FROM rustlang/rust:nightly-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig cmake make
ENV OPENSSL_NO_VENDOR=1
WORKDIR /app

# Cache dependencies — copy all workspace manifests
COPY Cargo.toml Cargo.lock ./
COPY ferrflow-wasm/Cargo.toml ferrflow-wasm/Cargo.toml

# Create stubs for all crates/bins so cargo resolves the workspace
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
    && mkdir -p benchmarks/fixtures && echo 'fn main() {}' > benchmarks/fixtures/generate.rs \
    && mkdir -p benches && echo 'fn main() {}' > benches/ferrflow_benchmarks.rs \
    && mkdir -p ferrflow-wasm/src && echo '' > ferrflow-wasm/src/lib.rs \
    && cargo build --release --package ferrflow \
    && rm -rf src benchmarks benches ferrflow-wasm/src

# Build for real
COPY src ./src
COPY benchmarks ./benchmarks
RUN mkdir -p ferrflow-wasm/src && echo '' > ferrflow-wasm/src/lib.rs \
    && cargo build --release --package ferrflow

# Runtime stage
FROM alpine:3.23
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/ferrflow /usr/local/bin/ferrflow
ENTRYPOINT ["ferrflow"]
CMD ["--help"]
