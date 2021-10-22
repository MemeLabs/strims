FROM golang:1.17 as build

WORKDIR /go/src/app
COPY . .
RUN export CGO_ENABLED=0 && go build -trimpath -mod=readonly ./cmd/svc


FROM alpine:latest

RUN apk update && \
    apk add ffmpeg && \
    rm -rf /var/cache/apk/*

WORKDIR /app
COPY --from=build /go/src/app/svc .
RUN adduser svc --disabled-password && echo "svc:" | chpasswd
USER svc

CMD ["/app/svc"]
