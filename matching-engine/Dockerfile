# Use the official Rust image as the base image
FROM rust:latest AS builder

# Set the working directory inside the container
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release

# Copy the rest of the source code
COPY . .

# Touch main.rs to invalidate cargo's cached dummy build
RUN touch src/main.rs

# Build the application
RUN cargo build --release

# Use a minimal base image with a newer glibc version
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends curl && rm -rf /var/lib/apt/lists/*

# Set the working directory inside the container
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/main .

# Config is volume-mounted at runtime
ENV CONFIG_PATH=/app/config.json

# Expose the port your server listens on
EXPOSE 8080

# Set the command to run the server
CMD ["./main"]
