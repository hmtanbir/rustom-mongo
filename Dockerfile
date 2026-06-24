# Stage 1: Planner
FROM lukemathwalker/cargo-chef:latest-rust-alpine AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Builder
FROM lukemathwalker/cargo-chef:latest-rust-alpine AS builder
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this layer will be cached unless dependencies change
RUN cargo chef cook --release --recipe-path recipe.json
# Copy actual code and build
COPY . .
RUN cargo build --release --bin rustom

# Stage 3: Runtime
FROM alpine:latest AS runtime
WORKDIR /app
# Install system packages needed at runtime (SSL certificates)
RUN apk add --no-cache \
    ca-certificates

# Copy build artifact from builder
COPY --from=builder /app/target/release/rustom /app/rustom


EXPOSE 3000
ENV TZ=Etc/UTC

CMD ["/app/rustom"]
