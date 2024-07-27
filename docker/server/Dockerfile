FROM rust:latest

# install git and redis
RUN apt-get update && apt-get install -y \
    git \
    redis-server \
 && apt-get clean \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /install

# clone from the trabas repo
RUN git clone https://github.com/amirkode/trabas.git .

# build now
RUN cargo build --release --manifest-path cli/Cargo.toml
RUN mkdir -p /install/bin \
 && cp target/release/trabas /install/bin/trabas

# copy the entry point
COPY entrypoint.sh /install/entrypoint.sh
RUN chmod +x /install/entrypoint.sh

# expose the public service port
EXPOSE 8787
# expose the client service port
EXPOSE 8789

# start services
ENTRYPOINT ["/install/entrypoint.sh"]
