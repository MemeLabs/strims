FROM golang:1.18 as build

ARG GIT_SHA
ARG BUILD_DATE

WORKDIR /go/src/app
COPY . .
RUN export CGO_ENABLED=0 && go build -trimpath -ldflags "-X 'main.GitSHA=$GIT_SHA' -X 'main.BuildDate=$BUILD_DATE'" ./infra/cmd/atmon


FROM alpine:latest

WORKDIR /app
COPY --from=build /go/src/app/atmon .

RUN addgroup -S appgroup && adduser -S appuser -G appgroup
USER appuser

ENTRYPOINT ["/app/atmon"]
