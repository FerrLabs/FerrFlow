FROM docker.io/library/rust:1.99-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update \
 && apt-get install -y --no-install-recommends git ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY ferrflow-wasm/Cargo.toml ferrflow-wasm/Cargo.toml
COPY api/Cargo.toml api/Cargo.toml
RUN mkdir -p src benches benchmarks/fixtures ferrflow-wasm/src api/src \
 && printf 'fn main() {}' > src/main.rs \
 && printf '' > src/lib.rs \
 && printf 'fn main() {}' > benches/ferrflow_benchmarks.rs \
 && printf 'fn main() {}' > benchmarks/fixtures/generate.rs \
 && printf '' > ferrflow-wasm/src/lib.rs \
 && printf 'fn main() {}' > api/src/main.rs \
 && printf '' > api/src/lib.rs \
 && cargo build --release --locked -p ferrflow-api --bin ferrflow-api \
 && rm -rf api/src

COPY api/openapi ./api/openapi
COPY api/src ./api/src
RUN touch api/src/main.rs api/src/lib.rs \
 && cargo build --release --locked -p ferrflow-api --bin ferrflow-api

FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app
COPY --from=builder /app/target/release/ferrflow-api /app/ferrflow-api

EXPOSE 3000
ENTRYPOINT ["/app/ferrflow-api"]
