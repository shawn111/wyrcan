FROM debian:latest

RUN apt-get update && \
    apt-get install --no-install-recommends -y linux-image-generic && \
    apt-get install --no-install-recommends -y ca-certificates && \
    apt-get install --no-install-recommends -y kexec-tools && \
    apt-get install --no-install-recommends -y buildah && \
    apt-get install --no-install-recommends -y systemd && \
    rm -rf /var/lib/apt/lists/*

RUN systemctl enable systemd-resolved.service systemd-networkd.service
RUN systemctl mask systemd-logind.service serial-getty@.service getty@.service

COPY autoconf.network /etc/systemd/network/autoconf.network
COPY wyrcan.service /etc/systemd/system/wyrcan.service
COPY wyrcan-extract /usr/local/bin/wyrcan-extract
COPY wyrcan-boot /usr/local/bin/wyrcan-boot

RUN systemctl enable wyrcan.service
RUN ln /bin/systemd /init
