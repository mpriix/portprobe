# Multi-stage enterprise container build for PortProbe
# Produces a minimal, non-root, hardened deployment container.

FROM rust:alpine AS builder

WORKDIR /usr/src/portprobe

# Install musl development tools
RUN apk add --no-cache musl-dev

# Copy manifests and pre-build dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src benches && \
    echo "fn main() {}" > src/main.rs && \
    touch src/lib.rs benches/concurrency_benchmark.rs benches/connection_benchmark.rs benches/throughput_benchmark.rs && \
    cargo build --release --bin portprobe && \
    rm -rf src benches target/release/deps/portprobe* target/release/deps/libportprobe* target/release/portprobe*

# Copy actual source files and build optimized binary
COPY src ./src
COPY benches ./benches
RUN touch src/main.rs src/lib.rs && cargo build --release --locked --bin portprobe

# Runtime stage: minimal non-root image
FROM alpine:3.20

# Create unprivileged runtime user
RUN addgroup -S -g 10001 portprobe && \
    adduser -S -u 10001 -G portprobe portprobe

# Copy binary from builder
COPY --from=builder /usr/src/portprobe/target/release/portprobe /usr/local/bin/portprobe

# Set permissions and user
RUN chmod 0755 /usr/local/bin/portprobe
USER portprobe:portprobe

# Configure default bind address and port in container environment (Finding SEC-FIND-21)
# Defaults to localhost (127.0.0.1) for safety; override with PORTPROBE_BIND=0.0.0.0 for container bridge networks.
ENV PORTPROBE_BIND=127.0.0.1
ENV PORTPROBE_PORT=8080

# Expose default non-privileged diagnostic port
EXPOSE 8080

# Native binary healthcheck for container orchestration (Finding SEC-FIND-06)
HEALTHCHECK --interval=30s --timeout=3s --retries=3 CMD /usr/local/bin/portprobe --health-check --port ${PORTPROBE_PORT:-8080} || exit 1

ENTRYPOINT ["/usr/local/bin/portprobe"]
CMD ["--port", "8080"]
