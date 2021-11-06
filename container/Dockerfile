FROM debian:latest

RUN apt update
RUN apt install -y linux-image-generic systemd
RUN apt install -y ca-certificates
RUN apt install -y kexec-tools
RUN apt install -y buildah

RUN systemctl enable systemd-resolved.service systemd-networkd.service
RUN systemctl mask systemd-logind.service serial-getty@.service getty@.service

COPY autoconf.network /etc/systemd/network/autoconf.network
COPY wyrcan.service /etc/systemd/system/wyrcan.service
COPY wyrcan-extract /usr/local/bin/wyrcan-extract
COPY wyrcan-boot /usr/local/bin/wyrcan-boot

RUN systemctl enable wyrcan.service
RUN ln /bin/systemd /init
