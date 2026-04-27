# ---------- Build stage ----------
FROM rust:1.89-bookworm AS builder
WORKDIR /app

# (Optionnel) dépendances système utiles pour certains crates
RUN apt-get update && apt-get install -y pkg-config && rm -rf /var/lib/apt/lists/*

# Workspace
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build le crate api
RUN cargo build -p api --release

# ---------- Runtime stage ----------
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/api /app/api

# Si ton code charge un .env obligatoire au démarrage, garder cette ligne.
# Sinon, la retirer.
COPY .env /app/.env

ENV ADDRESS=0.0.0.0:8080
EXPOSE 8080

CMD ["/app/api"]