FROM mcr.microsoft.com/vscode/devcontainers/base:ubuntu-20.04

# This Dockerfile adds a non-root user with sudo access. Use the "remoteUser"
# property in devcontainer.json to use it. On Linux, the container user's GID/UIDs
# will be updated to match your local UID/GID (when using the dockerFile property).
# See https://aka.ms/vscode-remote/containers/non-root-user for details.
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID

ARG NODE_VERSION=14.x
ARG GO_VERSION=1.17
ARG PROTOC_VERSION=3.15.8
ARG MEMELABS_PB_VERSION=0.1.17

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
  && cd /tmp/gotools \
  && GOPATH=/tmp/gotools GO111MODULE=on go get -v golang.org/x/tools/gopls@latest 2>&1 \
  && GOPATH=/tmp/gotools GO111MODULE=on go get -v \
    honnef.co/go/tools/...@latest \
    golang.org/x/tools/cmd/gorename@latest \
    golang.org/x/tools/cmd/goimports@latest \
    golang.org/x/tools/cmd/guru@latest \
    golang.org/x/lint/golint@latest \
    github.com/mdempsky/gocode@latest \
    github.com/cweill/gotests/...@latest \
    github.com/haya14busa/goplay/cmd/goplay@latest \
    github.com/sqs/goreturns@latest \
    github.com/josharian/impl@latest \
    github.com/davidrjenni/reftools/cmd/fillstruct@latest \
    github.com/uudashr/gopkgs/v2/cmd/gopkgs@latest \
    github.com/ramya-rao-a/go-outline@latest \
    github.com/acroca/go-symbols@latest \
    github.com/godoctor/godoctor@latest \
    github.com/rogpeppe/godef@latest \
    github.com/zmb3/gogetdoc@latest \
    github.com/fatih/gomodifytags@latest \
    github.com/mgechev/revive@latest \
    github.com/go-delve/delve/cmd/dlv@latest \
    github.com/golang/protobuf/protoc-gen-go 2>&1 \
  # Build go tools without module support
  && GOPATH=/tmp/gotools GO111MODULE=off go get -v github.com/alecthomas/gometalinter 2>&1 \
  # Build gocode-gomod
  && GOPATH=/tmp/gotools GO111MODULE=off go get -x -d github.com/stamblerre/gocode 2>&1 \
  && GOPATH=/tmp/gotools GO111MODULE=off go build -o gocode-gomod github.com/stamblerre/gocode \
  # Install go tools
  && mv /tmp/gotools/bin/* /usr/local/bin/ \
  && mv /tmp/gotools/gocode-gomod /usr/local/bin/ \
  # Clean up go path
  && cd ~ \
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
