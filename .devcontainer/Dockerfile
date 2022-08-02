FROM mcr.microsoft.com/vscode/devcontainers/base:ubuntu-20.04

# This Dockerfile adds a non-root user with sudo access. Use the "remoteUser"
# property in devcontainer.json to use it. On Linux, the container user's GID/UIDs
# will be updated to match your local UID/GID (when using the dockerFile property).
# See https://aka.ms/vscode-remote/containers/non-root-user for details.
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID

ARG NODE_VERSION=14.x
ARG GO_VERSION=1.19
ARG PROTOC_VERSION=3.20.1
ARG MEMELABS_PB_VERSION=0.3.4

# Avoid warnings by switching to noninteractive
ENV DEBIAN_FRONTEND=noninteractive

RUN \
  # Create a non-root user to use if not already available
  if [ $(getent passwd $USERNAME) ]; then \
    # If exists, see if we need to tweak the GID/UID
    if [ "$USER_GID" != "1000" ] || [ "$USER_UID" != "1000" ]; then \
      groupmod --gid $USER_GID $USERNAME \
      && usermod --uid $USER_UID --gid $USER_GID $USERNAME \
      && chown -R $USER_UID:$USER_GID /home/$USERNAME; \
    fi; \
  else \
    # Otherwise create the non-root user
    groupadd --gid $USER_GID $USERNAME \
    && useradd -s /bin/bash --uid $USER_UID --gid $USER_GID -m $USERNAME \
    # Add sudo support for the non-root user
    && apt-get install -y sudo \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME\
    && chmod 0440 /etc/sudoers.d/$USERNAME; \
  fi \
  # Install deps
  && apt-get update \
  && apt-get install -y build-essential ffmpeg \
  # Install go
  && curl -LO "https://golang.org/dl/go${GO_VERSION}.linux-amd64.tar.gz" \
  && tar -C /usr/local -xzf "go${GO_VERSION}.linux-amd64.tar.gz" \
  && ln -s /usr/local/go/bin/go /usr/local/bin \
  && rm "go${GO_VERSION}.linux-amd64.tar.gz" \
  # Build go tools with module support
  && mkdir -p /tmp/gotools \
  && GOPATH=/tmp/gotools go install golang.org/x/tools/gopls@latest 2>&1 \
  && GOPATH=/tmp/gotools go install github.com/cweill/gotests/...@latest 2>&1 \
  && GOPATH=/tmp/gotools go install github.com/fatih/gomodifytags@latest 2>&1 \
  && GOPATH=/tmp/gotools go install github.com/josharian/impl@latest 2>&1 \
  && GOPATH=/tmp/gotools go install github.com/haya14busa/goplay/cmd/goplay@latest 2>&1 \
  && GOPATH=/tmp/gotools go install github.com/go-delve/delve/cmd/dlv@latest 2>&1 \
  && GOPATH=/tmp/gotools go install honnef.co/go/tools/cmd/staticcheck@latest 2>&1 \
  && GOPATH=/tmp/gotools go install google.golang.org/protobuf/cmd/protoc-gen-go@latest 2>&1 \
  && rm -rf /tmp/gotools \
  # Install golangci-lint
  && curl -sSfL https://raw.githubusercontent.com/golangci/golangci-lint/master/install.sh | sh -s -- -b /usr/local/bin 2>&1 \
  # Set up go directories
  && mkdir -p "/${USERNAME}/go/src" "/${USERNAME}/go/bin" "/${USERNAME}/go/pkg/mod" \
  && chmod -R 777 "/${USERNAME}/go" \
  # Install node.js
  && curl -sL "https://deb.nodesource.com/setup_${NODE_VERSION}" | bash - \
  && apt-get install -y nodejs \
  # Install protoc
  && curl -LO "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip" \
  && unzip "protoc-${PROTOC_VERSION}-linux-x86_64.zip" -d /usr/local \
  && chmod 755 /usr/local/bin/protoc \
  && chmod -R 755 /usr/local/include/google \
  && chmod 644 /usr/local/include/google/protobuf/* \
  && rm "protoc-${PROTOC_VERSION}-linux-x86_64.zip" \
  # Install protoc plugins
  && curl -LO "https://github.com/MemeLabs/protobuf/releases/download/v${MEMELABS_PB_VERSION}/protoc-plugins-${MEMELABS_PB_VERSION}-linux-x86_64.tar.gz" \
  && tar -C /usr/local/bin --strip-components=1 -xzf "protoc-plugins-${MEMELABS_PB_VERSION}-linux-x86_64.tar.gz" \
  && chmod -R 755 /usr/local/bin/protoc-* \
  && rm "protoc-plugins-${MEMELABS_PB_VERSION}-linux-x86_64.tar.gz" \
  # Clean up
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*

ENV GOROOT=/usr/local/go
ENV GOPATH="/${USERNAME}/go"
ENV GO111MODULE=auto
ENV PATH="${GOPATH}/bin:${GOROOT}/bin:${PATH}"

# switch back to dialog for any ad-hoc use of apt-get
ENV DEBIAN_FRONTEND=dialog
