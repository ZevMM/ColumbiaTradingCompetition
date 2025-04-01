# Use the official Rust image as the base image
FROM rust:latest AS builder

# Set the working directory inside the container
WORKDIR /app

# Copy the Cargo.toml, Cargo.lock, build.rs, and config.json files
COPY Cargo.toml Cargo.lock build.rs config.json ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release

# Copy the rest of the source code
COPY . .

# Build the application
RUN cargo build --release

# Use a minimal base image with a newer glibc version
FROM debian:bookworm-slim

# Set the working directory inside the container
WORKDIR /app

# Copy the compiled binary and config.json from the builder stage
COPY --from=builder /app/target/release/main .
COPY --from=builder /app/config.json .

# Expose the port your server listens on
EXPOSE 8080

# Set the command to run the server
CMD ["./main"]