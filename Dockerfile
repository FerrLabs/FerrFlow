# Build stage
FROM rust:1.99-alpine@sha256:b1d5dd5a22a09404d3e798c0cf051b689077360374eb505fa5bb93c479693d87 AS builder
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
COPY schema ./schema
COPY benchmarks ./benchmarks
RUN mkdir -p benches && echo 'fn main() {}' > benches/ferrflow_benchmarks.rs \
    && mkdir -p ferrflow-wasm/src && echo '' > ferrflow-wasm/src/lib.rs \
    && touch src/main.rs src/lib.rs \
    && cargo build --release --package ferrflow

# Runtime stage
FROM alpine:3.24@sha256:be2656061b536c1883c4034790e21d0a09c5aaaf51968b89368a3c0371ea2c9a
RUN apk add --no-cache ca-certificates git \
    && git config --system --add safe.directory '*' \
    && git config --system user.name FerrFlow \
    && git config --system user.email bot@ferrflow.com \
    && adduser -D -u 1000 ferrflow
COPY --from=builder /app/target/release/ferrflow /usr/local/bin/ferrflow
USER ferrflow
WORKDIR /repo
ENTRYPOINT ["ferrflow"]
CMD ["--help"]
