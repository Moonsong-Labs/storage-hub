# STORAGE_HUB_MSP_BACKEND DOCKERFILE
#
# Requires to run from root folder and to copy the binary in the build folder
# (This can be done as part of the release workflow or manually)

FROM ubuntu:noble

LABEL version="0.1.0"
LABEL description="Storage Hub MSP Backend"

ENV RUST_BACKTRACE=1

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    ca-certificates curl sudo libpq-dev && \
    apt-get autoremove -y && \
    apt-get clean && \
    find /var/lib/apt/lists/ -type f -not -name lock -delete && \
    useradd -m -u 1337 -U -s /bin/sh -d /backend backend && \
    echo "backend ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

USER backend

COPY --chown=backend:backend build/sh-msp-backend /usr/local/bin/sh-msp-backend
RUN chmod uog+x /usr/local/bin/sh-msp-backend

EXPOSE 8080

ENTRYPOINT ["sh-msp-backend"]