# =========================================================================
# ---- Stage 1: Builder ----
# This stage compiles the Rust application.
# =========================================================================

# App name from compose file or build args.
# This allows you to specify the name of the binary dynamically.
# You can pass this as a build argument when building the Docker image.
# Example: docker build --build-arg APP_NAME=your_app_name .
ARG APP_NAME=rust-poem-server

FROM rust:1-slim-bookworm AS builder

# Install system dependencies required for compiling common Rust crates.
# 'libssl-dev' is often needed for crates that use OpenSSL.
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev

# Set the working directory inside the container.
WORKDIR /usr/src/app

# Copy the dependency manifest files.
COPY Cargo.toml Cargo.lock ./

# Now, copy the actual application source code.
COPY ./templates ./templates
COPY ./static ./static
COPY ./migration ./migration
# Create a dummy project and build it to cache dependencies.
# This leverages Docker's layer caching. As long as Cargo.toml and Cargo.lock
# don't change, the dependencies won't be re-downloaded or re-compiled
# on subsequent builds, speeding up the process significantly.
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release

# Now, copy the actual application source code.
COPY ./src ./src


# Touch the main source file to ensure Cargo recognizes the code change
# and rebuilds the final binary, not the dummy one.
RUN touch src/main.rs

# Build the application for release.
# The --release flag enables optimizations for a production build.
RUN cargo build --release


# =========================================================================
# ---- Stage 2: Runner ----
# This stage creates the final, lightweight image to run the application.
# =========================================================================
FROM debian:stable-slim AS final

ARG APP_NAME=rust-poem-server

# Ensure all security updates are applied
RUN apt-get update && apt-get upgrade -y --no-install-recommends && rm -rf /var/lib/apt/lists/*

# Install runtime dependencies.
# 'ca-certificates' is ESSENTIAL for your application to trust the TLS
# certificate of the mail server you're connecting to on port 465.
# Without this, you would get TLS/SSL handshake errors.
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root user and group for security best practices.
# Running as a non-root user minimizes potential damage from a security vulnerability.
RUN groupadd --system app && useradd --system --gid app app
USER app

# Set the working directory for the non-root user.
WORKDIR /home/app

# Copy the compiled binary from the 'builder' stage.
# IMPORTANT: Replace 'rust-poem-server' with the actual name of your
# binary as defined in your Cargo.toml file (usually the package name).
COPY --from=builder /usr/src/app/target/release/${APP_NAME} ./server
# Copy the static files and templates to the working directory.
COPY --from=builder /usr/src/app/templates ./templates
COPY --from=builder /usr/src/app/static ./static

# Expose the port your Poem web server listens on.
# This doesn't publish the port, but documents which port should be published.
EXPOSE 5000

# Set the command to run the application when the container starts.
CMD ["./server"]
