FROM alpine:latest

RUN apk --no-cache add ffmpeg

RUN adduser svc --disabled-password && echo "svc:" | chpasswd
USER svc
