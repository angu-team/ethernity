FROM rust:1.87-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        curl \
        git \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install anvil from Foundry
RUN curl -L https://foundry.paradigm.xyz | bash \
    && /root/.foundry/bin/foundryup

ENV PATH="/root/.foundry/bin:${PATH}"

COPY . .

CMD ["cargo", "run", "-p", "sandwich-victim", "--example", "mempool_watch", "--", "ws://148.251.183.245:8546"]
