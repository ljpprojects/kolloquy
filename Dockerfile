FROM rust:1.86.0-bookworm AS server-builder

COPY . .

USER root
RUN apt-get install pkg-config
RUN apt-get install libssl-dev

WORKDIR /server/
RUN ["cargo", "build", "--release"]

FROM node:23.11.0-bookworm-slim as client-builder

RUN npm install typescript --global
COPY . .

COPY tsconfig.json ./

RUN tsc

FROM debian:bookworm-slim

RUN groupadd -r kolloquy

COPY --from=server-builder /server/target/release/server /server/bin
COPY --from=client-builder /client/dist /client/dist

RUN chmod 750 /server/bin
RUN chgrp kolloquy /server/bin

RUN setcap 'cap_net_bind_service=+ep' /server/bin
RUN mkdir /server/logs/
RUN touch /server/logs/log.txt

WORKDIR /server
EXPOSE 80
CMD ["./bin"]