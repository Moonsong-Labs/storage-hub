# STORAGE_HUB_NODE DOCKERFILE
#
# Requires to run from /test folder and to copy the binary in the build folder
# (This can be done as part of the release workflow or manually)

FROM ubuntu:noble

LABEL version="0.1.0"
LABEL description="Storage Hub Node"

ENV RUST_BACKTRACE=1

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    ca-certificates curl sudo librocksdb-dev libpq-dev && \
    apt-get autoremove -y && \
    apt-get clean && \
    find /var/lib/apt/lists/ -type f -not -name lock -delete && \
    useradd -m -u 1337 -U -s /bin/sh -d /storage-hub storage-hub && \
    mkdir -p /data /storage-hub/.local/share /specs /storage && \
    chown -R storage-hub:storage-hub /data && \
    ln -s /data /storage-hub/.local/share/storage-hub-node && \
    echo "storage-hub ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers && \
    chmod -R 777 /storage /data

USER storage-hub

COPY --chown=storage-hub:storage-hub build/storage-hub-node /usr/local/bin/storage-hub-node
RUN chmod uog+x /usr/local/bin/storage-hub-node

EXPOSE 9333 9944 30333 30334 9615

VOLUME ["/data"]

ENTRYPOINT ["storage-hub-node"]
CMD ["--tmp"]