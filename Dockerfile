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

# Install dasel to inspect the app's name because Cargo insists on building
# binaries with the binary name specified in Cargo.toml and there's no way to
# currently customize that behavior
#
# We install it into utils and verify that its checksum didn't change since
# the creation of this script to prevent potential supply chain attacks
RUN mkdir utils
WORKDIR ./utils
RUN curl -o dasel -L \
  "https://github.com/TomWright/dasel/releases/download/v1.27.3/dasel_linux_amd64"
RUN echo "1a5adbf8e5b69f48ad5d1665bf7ed056ea3ff8cf3312ce2dc7c3209939873489  dasel" \
  | sha256sum --status --check
RUN chmod +x dasel
WORKDIR ../

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

# Rename project binary to app to simplify copying it in next stage
RUN mv "target/x86_64-unknown-linux-musl/release/$(/utils/dasel select -f Cargo.toml .package.name)" \
  target/x86_64-unknown-linux-musl/release/app

# Establish an alphine container.
# This is where we will actually run our code, we can use the scratch Docker
# image if we wanted, but there are a couple of nice to have utilities inside
# of alphine that are nice to have for debugging (like an actual shell lol)
FROM alpine:latest

# You can create ARG/ENV line pairs here for each environment var
ARG GITHUB_TOKEN
ENV GITHUB_TOKEN=$GITHUB_TOKEN
ARG MAGIC_NUMBER
ENV MAGIC_NUMBER=$MAGIC_NUMBER

# Copy the statically-linked binary from the build image
COPY --from=build \
  /app/target/x86_64-unknown-linux-musl/release/app \
  /usr/local/bin/app

# If your app relies on any external files at runtime, you should create an
# assets folder and put them there and uncomment this line
COPY ./assets/ /home/assets/

WORKDIR /home
CMD ["/usr/local/bin/app"]
