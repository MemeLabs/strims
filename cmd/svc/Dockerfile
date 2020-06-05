FROM golang:1.14 as build

WORKDIR /go/src/app

COPY . . 

RUN go build -trimpath ./cmd/svc


FROM ubuntu:18.04

WORKDIR /app

COPY --from=build /go/src/app/svc .

RUN useradd -m svc

USER svc

CMD ["/app/svc"]
