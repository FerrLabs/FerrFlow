FROM docker.io/library/rust:1.97-slim-bookworm AS builder

WORKDIR /app

# `git` is needed at build time: the `ferrflow` dependency is a pinned git
# revision. Depuis l'ajout de `ferrlabs-api-version`, la construction tire
# aussi une crate du registre privé Kellnr — d'où le `.cargo/config.toml`
# copié plus bas et le secret monté sur les deux `cargo build`.
RUN apt-get update \
 && apt-get install -y --no-install-recommends git ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Index Kellnr + fournisseur d'identifiants. Sans ce fichier, cargo échoue sur
# `registry index was not found in any configuration: kellnr` dès le parsing
# du manifeste — avant même de tenter le réseau.
COPY api/.cargo ./.cargo
COPY api/Cargo.toml api/Cargo.lock ./
COPY api/rust-toolchain.toml ./
RUN --mount=type=secret,id=cargo-registry-token \
    mkdir -p src \
 && printf 'fn main() {}' > src/main.rs \
 && printf '' > src/lib.rs \
 && CARGO_REGISTRIES_KELLNR_TOKEN="$(cat /run/secrets/cargo-registry-token)" \
    cargo build --release --bin ferrflow-api \
 && rm -rf src

COPY api/openapi ./openapi
COPY api/src ./src
RUN --mount=type=secret,id=cargo-registry-token \
    touch src/main.rs src/lib.rs \
 && CARGO_REGISTRIES_KELLNR_TOKEN="$(cat /run/secrets/cargo-registry-token)" \
    cargo build --release --bin ferrflow-api

FROM gcr.io/distroless/cc-debian12:nonroot

WORKDIR /app
COPY --from=builder /app/target/release/ferrflow-api /app/ferrflow-api

EXPOSE 3000
ENTRYPOINT ["/app/ferrflow-api"]
