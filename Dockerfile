##########
# This stage builds the rust binaries
##########
FROM rust:1.73 as builder

ENV BASE_DIR=/findtime

# Create a new empty package
WORKDIR $BASE_DIR
RUN cargo new --bin rust
WORKDIR $BASE_DIR/rust

# Copy over the manifests
COPY Cargo.lock Cargo.toml ./

# This build step will cache the dependencies
RUN cargo build --release

COPY src ./src

COPY index.html ./index.html

# Build binaries
RUN cargo build --release

CMD ADDRESS="0.0.0.0" ./target/release/findtime
# Copy artefacts to output folder

# FROM rust:1.73 AS runtime
# 
# ENV BASE_DIR=/findtime
# WORKDIR $BASE_DIR
# 
# COPY --from=builder  /findtime/rust/target/release/findtime /findtime/
# 
# COPY index.html ./index.html
# 
# CMD ADDRESS="0.0.0.0" ./findtime/findtime
