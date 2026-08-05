FROM docker.io/library/rust:1.97-slim-bookworm AS builder

WORKDIR /app

# `git` is needed at build time: the `ferrflow` dependency is a pinned git
# revision, not a registry crate. Nothing here talks to a private registry,
# so this image builds anywhere without a Kellnr token.
RUN apt-get update \
 && apt-get install -y --no-install-recommends git ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY api/Cargo.toml api/Cargo.lock ./
COPY api/rust-toolchain.toml ./
RUN mkdir -p src \
 && printf 'fn main() {}' > src/main.rs \
 && printf '' > src/lib.rs \
 && cargo build --release --bin ferrflow-api \
 && rm -rf src

COPY api/openapi ./openapi
COPY api/src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release --bin ferrflow-api

FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app
COPY --from=builder /app/target/release/ferrflow-api /app/ferrflow-api

EXPOSE 3000
ENTRYPOINT ["/app/ferrflow-api"]
