FROM ubuntu:focal AS toolchain

ARG NODE_VERSION=26.11.1

# install packages at the start so changing source files doesn't taint the layer cache
RUN \
  # Install deps
  apt-get update \
  && apt-get install -y build-essential git-core curl brotli \
  # Install node.js
  && curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.gz" \
    | tar -xz -C /usr/local --strip-components=1

# At least the workspace's rust-version. RUSTUP_TOOLCHAIN overrides
# rust-toolchain.toml's floating stable channel, so every cargo call uses this
# release and needs only its wasm target.
ARG RUST_VERSION=1.99.0

ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_TOOLCHAIN=${RUST_VERSION}
ENV PATH="${CARGO_HOME}/bin:${PATH}"

RUN \
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --profile minimal \
      --default-toolchain "${RUST_VERSION}" \
      --target wasm32-unknown-unknown

# must match strims-wasm's wasm-bindgen pin
ARG WASM_BINDGEN_VERSION=0.2.129

RUN cargo install wasm-bindgen-cli --version "${WASM_BINDGEN_VERSION}" --locked

FROM toolchain AS rust-deps

WORKDIR /ui

COPY \
  Cargo.toml \
  Cargo.lock \
  rust-toolchain.toml \
  ./
COPY .cargo ./.cargo
# strims-proto's build script compiles the protos in schema/
COPY schema ./schema
COPY crates ./crates
# cache the crates strims-wasm builds from; build scripts and proc macros
# compile for the host, so its dependencies are fetched too
RUN cargo fetch --locked --target wasm32-unknown-unknown --target host-tuple

FROM rust-deps AS build

ARG VERSION=development

COPY .git ./.git
COPY assets ./assets
COPY src ./src
COPY webpack ./webpack
COPY vendor_modules ./vendor_modules
COPY \
  package-lock.json \
  package.json \
  tsconfig.json \
  webpack.config.js \
  ./

RUN \
  # Build ui; the strims-wasm loader runs cargo and wasm-bindgen
  npm ci \
  && npm run build \
  && find dist/web -type f | xargs gzip -k9 \
  && find dist/web -not -name "*.gz" -type f | xargs brotli

FROM ghcr.io/memelabs/strims/nginx:latest

COPY hack/web/nginx.conf /etc/nginx/nginx.conf
COPY --from=build /ui/dist/web /usr/share/nginx/html/
