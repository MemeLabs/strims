FROM ubuntu:focal AS build

ARG VERSION=development

ARG NODE_VERSION=14.x
ARG GO_VERSION=1.19

ENV GOROOT=/usr/local/go
ENV GOPATH="/tmp/go"
ENV GO111MODULE=auto
ENV PATH="${GOPATH}/bin:${GOROOT}/bin:${PATH}"

# install packages at the start so changing soure files doesn't taint the layer cache
RUN \
  # Install deps
  apt-get update \
  && apt-get install -y build-essential git-core curl brotli \
  # Install go
  && curl -LO "https://golang.org/dl/go${GO_VERSION}.linux-amd64.tar.gz" \
  && tar -C /usr/local -xzf "go${GO_VERSION}.linux-amd64.tar.gz" \
  && ln -s /usr/local/go/bin/go /usr/local/bin \
  && rm "go${GO_VERSION}.linux-amd64.tar.gz" \
  # Set up go directories
  && mkdir -p "${GOPATH}/src" "${GOPATH}/bin" "${GOPATH}/pkg/mod" \
  && chmod -R 777 "${GOPATH}" \
  # Install node.js
  && curl -sL "https://deb.nodesource.com/setup_${NODE_VERSION}" | bash - \
  && apt-get install -y nodejs

RUN mkdir /ui
WORKDIR /ui

COPY \
  go.mod \
  go.sum \
  ./
# cache go modules
RUN go mod download

COPY .git ./.git
COPY assets ./assets
COPY src ./src
COPY webpack ./webpack
COPY vendor_modules ./vendor_modules
COPY pkg ./pkg
COPY internal ./internal
COPY \
  .eslintrc.js \
  package-lock.json \
  package.json \
  tsconfig.json \
  webpack.config.js \
  ./

RUN \
  # Build ui
  npm install --unsafe-perm \
  && npm run build \
  && find dist/web -type f | xargs gzip -k9 \
  && find dist/web -not -name "*.gz" -type f | xargs brotli

FROM ghcr.io/memelabs/strims/nginx:latest

COPY hack/web/nginx.conf /etc/nginx/nginx.conf
COPY --from=build /ui/dist/web /usr/share/nginx/html/
