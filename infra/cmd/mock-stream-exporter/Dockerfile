FROM golang:1.19 as build

ARG GIT_SHA
ARG BUILD_DATE

WORKDIR /go/src/app
COPY . .
RUN export CGO_ENABLED=0 && go build -trimpath ./infra/cmd/mock-stream-exporter


FROM alpine:latest

WORKDIR /app
COPY --from=build /go/src/app/mock-stream-exporter .

RUN addgroup -S appgroup && adduser -S appuser -G appgroup
USER appuser

ENTRYPOINT ["/app/mock-stream-exporter"]
