FROM debian:stretch

ARG TYPE=debug
#RUN apt-get update && apt-get -y install libsodium18 libsodium-dev pkg-config
RUN apt update && apt full-upgrade -y && apt install -y libc++-dev curl libssl1.1 zlib1g-dev zlibc
WORKDIR /src/app
COPY ./target/$TYPE/dmbc-node /src/app/
COPY ./target/$TYPE/dmbc-discovery /src/app/
RUN mkdir /src/app/etc
COPY ./etc/config.toml /src/app/etc/config.toml
RUN mkdir -p /src/app/var/db
RUN mkdir -p /src/app/var/keys
COPY ./var/keys/consensus /src/app/var/keys/consensus
COPY ./var/keys/consensus /src/app/var/keys/consensus.pub
COPY ./var/keys/consensus /src/app/var/keys/service
COPY ./var/keys/consensus /src/app/var/keys/service.pub

RUN chmod +x /src/app/dmbc-node
RUN chmod +x /src/app/dmbc-discovery

CMD ["/src/app/dmbc-node"]
