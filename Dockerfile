# Name your project the same as the final binary's name
# This needs to be updated for each project
#
# TODO: Get the project name from Cargo.toml while running instead of this mess
ARG PROJECT_NAME=rust-is-beautiful

# The default rust Docker container without any extra bells and whistles
# It's going to be used only for building the application because it's
# a full Debian OS which is overkill for running compiled Rust applications
FROM rust:1.64.0 AS build

# Install musl-tools which is provides the `musl-gcc` necessary to compile:
# - ring
# If your dependency tree doesn't have any of the above packages feel free to
# comment this
RUN apt update
RUN apt-get install -y musl-tools

# Create a dummy project and build the app's dependencies.
# If the Cargo.toml or Cargo.lock files have not changed,
# we can use the docker build cache and skip these (typically slow) steps.
# Credit goes to:
# https://alexbrand.dev/post/how-to-package-rust-applications-into-minimal-docker-containers/
RUN cargo new app
WORKDIR ./app
COPY Cargo.toml Cargo.lock ./
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --target=x86_64-unknown-linux-musl --release

# Copy the source and build the application.
COPY src ./src
# !!! THIS STEP IS ESSETNAIL TO ENSURE THAT CARGO WILL NOT SKIP THE BUILD !!!
# Sadly cargo uses mtime right now and so it will skip the build unless we
# force rehydrate the mtime of the file
RUN touch src/main.rs
RUN cargo build --target=x86_64-unknown-linux-musl --release

# Establish an alphine container.
# This is where we will actually run our code, we can use the scratch Docker
# image if we wanted, but there are a couple of nice to have utilities inside
# of alphine that are nice to have for debugging (like an actual shell lol)
FROM alpine:latest

# To continue having access to our argument PROJECT_NAME declared previously
# we need to "redeclare it" here
#
# TODO: This will be unnecessary if we got rid of needing to specify the name
ARG PROJECT_NAME

# You can create ARG/ENV line pairs here for each environment var
ARG GITHUB_TOKEN
ENV GITHUB_TOKEN=$GITHUB_TOKEN
ARG MAGIC_NUMBER
ENV MAGIC_NUMBER=$MAGIC_NUMBER

# Copy the statically-linked binary from the build image
COPY --from=build \
  /app/target/x86_64-unknown-linux-musl/release/$PROJECT_NAME \
  /usr/local/bin/app

# If your app relies on any external files at runtime, you should create an
# assets folder and put them there and uncomment this line
COPY ./assets/ /home/assets/

WORKDIR /home
CMD ["/usr/local/bin/app"]
