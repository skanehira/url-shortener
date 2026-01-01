# Build stage
FROM rust:1.92-slim-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

# Planner stage
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates/shortener-core/Cargo.toml crates/shortener-core/
COPY crates/shortener-service/Cargo.toml crates/shortener-service/
COPY crates/analytics-service/Cargo.toml crates/analytics-service/
# Create dummy source files for cargo metadata
RUN mkdir -p crates/shortener-core/src && touch crates/shortener-core/src/lib.rs && \
    mkdir -p crates/shortener-service/src && touch crates/shortener-service/src/main.rs && \
    mkdir -p crates/analytics-service/src && touch crates/analytics-service/src/main.rs
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage
FROM chef AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

ARG SERVICE_NAME
ENV SERVICE_NAME=${SERVICE_NAME}

RUN cargo build --release --package ${SERVICE_NAME}

# Runtime stage - using distroless for security
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

# Copy binary with correct ownership (nonroot user has uid 65532)
ARG SERVICE_NAME
COPY --from=builder --chown=65532:65532 /app/target/release/${SERVICE_NAME} /app/service

# Use nonroot user (provided by distroless)
USER nonroot:nonroot

EXPOSE 8080

ENTRYPOINT ["/app/service"]
