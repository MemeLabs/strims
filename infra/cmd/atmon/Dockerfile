FROM golang:1.14 as build

WORKDIR /go/src/app
COPY . .
RUN export CGO_ENABLED=0 && go build -trimpath ./infra/cmd/atmon


FROM alpine:latest

WORKDIR /app
COPY --from=build /go/src/app/atmon .

RUN addgroup -S appgroup && adduser -S appuser -G appgroup
USER appuser

ENTRYPOINT ["/app/atmon"]
