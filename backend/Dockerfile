# --- Builder ---
FROM docker.io/library/rust:1.79-slim AS builder
WORKDIR /app

ARG LLVM_VERSION=15
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang-${LLVM_VERSION} \
    llvm-${LLVM_VERSION}-dev \
    libclang-${LLVM_VERSION}-dev \
    build-essential \
    pkg-config \
    protobuf-compiler \
    libpq-dev \
    ca-certificates \
  && rm -rf /var/lib/apt/lists/*

ENV LIBCLANG_PATH=/usr/lib/llvm-${LLVM_VERSION}/lib

COPY . .

RUN cargo build --release --locked -p sh-msp-backend --features mocks

# --- Runtime ---
FROM docker.io/library/debian:bookworm-slim
WORKDIR /app
ENV RUST_LOG=info

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/sh-msp-backend /usr/local/bin/sh-msp-backend

EXPOSE 8080

CMD ["/usr/local/bin/sh-msp-backend", "--host", "0.0.0.0", "--port", "8080"]
