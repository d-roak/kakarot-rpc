FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json

# Build application
COPY . .
# Install system dependencies
RUN apt-get update && \
    apt-get -y upgrade && \ 
    apt-get install -y libclang-dev
RUN cargo build --release --bin kakarot-rpc

FROM debian:bookworm-slim AS runtime
WORKDIR /usr/app
COPY --from=builder /app/target/release/kakarot-rpc /usr/local/bin

# Expose the port that the RPC server will run on
EXPOSE 9545
EXPOSE 3030

# this is required to have exposing ports work from docker, the default is not this.
ENV KAKAROT_RPC_URL="0.0.0.0:3030"

# Add a health check to make sure the service is healthy
HEALTHCHECK --interval=3s --timeout=5s --start-period=1s --retries=5 \
    CMD curl --request POST \
    --header "Content-Type: application/json" \
    --data '{"jsonrpc": "2.0", "method": "eth_chainId", "id": 1}' http://${KAKAROT_RPC_URL} || exit 1

RUN apt-get update && \
    apt-get -y upgrade && \ 
    apt-get install -y libssl-dev ca-certificates tini curl && \ 
    rm -rf /var/lib/apt/lists/*

# Seen in https://github.com/eqlabs/pathfinder/blob/4ab915a830953ed6f02af907937b46cb447d9a92/Dockerfile#L120 - 
# Allows for passing args down to the underlying binary easily
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/kakarot-rpc"]

# empty CMD is needed and cannot be --help because otherwise configuring from
# environment variables only would be impossible and require a workaround.
CMD []
