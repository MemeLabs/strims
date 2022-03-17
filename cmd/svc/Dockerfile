FROM golang:1.18 as build

WORKDIR /go/src/app
COPY . .
RUN CGO_ENABLED=0 GOAMD64=v3 go build -trimpath -mod=readonly ./cmd/svc


FROM alpine:latest

RUN apk update && \
    apk add ffmpeg && \
    rm -rf /var/cache/apk/*

WORKDIR /app
COPY --from=build /go/src/app/svc .
RUN adduser svc --disabled-password && echo "svc:" | chpasswd
USER svc

ENTRYPOINT ["/app/svc"]
CMD ["run"]
