FROM golang:1.18beta2 as build

WORKDIR /go/src/app
COPY . .
RUN export CGO_ENABLED=0 && go build -trimpath -mod=readonly ./infra/cmd/kube-node-labels


FROM alpine:latest

WORKDIR /app
COPY --from=build /go/src/app/kube-node-labels .

RUN addgroup -g 1001 -S user
RUN adduser -S user -u 1001
RUN chown user:user /app/kube-node-labels
USER user

ENTRYPOINT /app/kube-node-labels
