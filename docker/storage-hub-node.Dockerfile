# STORAGE_HUB_NODE DOCKERFILE
#
# Requires to run from repository root and to copy the binary in the build folder
# (This can be done as part of the release workflow or manually)

FROM docker.io/library/ubuntu:rolling AS builder

# show backtraces
ENV RUST_BACKTRACE 1

# install tools and dependencies
RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
	    ca-certificates && \
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
	useradd -m -u 1337 -U -s /bin/sh -d /storage-hub storage-hub && \
	mkdir -p /data /storage-hub/.local/share && \
	chown -R storage-hub:storage-hub /data && \
	ln -s /data /storage-hub/.local/share/storage-hub-node && \
	mkdir -p /specs

USER storage-hub

# copy the compiled binary to the container
COPY --chown=storage-hub:storage-hub --chmod=774 build/storage-hub-node /usr/bin/storage-hub-node

# check if executable works in this container
RUN /usr/bin/storage-hub-node --version

# ws_port
EXPOSE 9333 9944 30333 30334

CMD ["/usr/bin/storage-hub-node"]
