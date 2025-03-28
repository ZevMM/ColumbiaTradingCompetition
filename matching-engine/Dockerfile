# Use the official Rust image as the base image
FROM rust:1.72 AS builder

# Set the working directory inside the container
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release && rm -rf src

# Copy the source code
COPY . .

# Build the application
RUN cargo build --release

# Use a minimal base image for the final container
FROM debian:buster-slim

# Set the working directory inside the container
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/matching-engine .

# Expose the port your server listens on
EXPOSE 8080

# Set the command to run the server
CMD ["./matching-engine"]