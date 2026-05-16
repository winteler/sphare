FROM debian:bookworm-slim

# Install any needed runtime libs
RUN apt-get update && apt-get install -y --no-install-recommends libssl3 ca-certificates rsync && apt-get clean

# Copy binary from builder
COPY ./target/release/server /usr/local/bin/sphare
COPY ./target/release/hash.txt /usr/local/bin/hash.txt
COPY ./target/site /usr/local/bin/site

RUN chmod +x /usr/local/bin/sphare

ENV LEPTOS_OUTPUT_NAME="sphare"
ENV LEPTOS_SITE_ROOT="/usr/local/bin/site"
ENV LEPTOS_SITE_PKG_DIR="pkg"
ENV LEPTOS_SITE_ADDR="0.0.0.0:3000"
ENV LEPTOS_ENV="PROD"
ENV LEPTOS_HASH_FILES="true"

EXPOSE 3000
ENTRYPOINT ["sphare"]