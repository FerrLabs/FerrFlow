# Build stage
FROM rust:1.98-alpine@sha256:7cc1c22d77d9432f7fe012a70e6d3e555af54c2a6832700ed7d553f1769ae89f AS builder
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
RUN mkdir -p benches && echo 'fn main() {}' > benches/ferrflow_benchmarks.rs \
    && mkdir -p ferrflow-wasm/src && echo '' > ferrflow-wasm/src/lib.rs \
    && cargo build --release --package ferrflow

# Runtime stage
FROM alpine:3.24@sha256:294b683cb724975bec92580e1e685676bd4b50bda910ddb8c51d4cabeaec77e6
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
