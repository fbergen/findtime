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

RUN rm ./src/*.rs

COPY src ./src
# Build binaries
RUN rm ./target/release/deps/findtime*

RUN cargo build --release

FROM node:alpine as jsbuilder

RUN npm install uglify-js -g

COPY ./main.js /build/main.js
RUN uglifyjs /build/main.js -o /build/main.js --compress --mangle

# CMD ADDRESS="0.0.0.0" ./target/release/findtime
# Copy artefacts to output folder

# FROM rust:1.73-slim-bookworm
FROM debian:bookworm-slim

ENV BASE_DIR=/findtime
WORKDIR $BASE_DIR

RUN apt-get update; \
    apt-get install -y --no-install-recommends \
        ca-certificates;

COPY --from=builder  /findtime/rust/target/release/findtime /findtime/
COPY --from=jsbuilder /build/main.js ./main.js
COPY index.html ./index.html

CMD ADDRESS="0.0.0.0" ./findtime
