FROM ubuntu:20.04 as builder
ENV TZ=Europe/Stockholm
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone
RUN apt-get -y update && apt-get -y install protobuf-compiler curl git libssl-dev libudev-dev make pkg-config zlib1g-dev llvm clang cmake openssh-client
RUN mkdir -p -m 0700 ~/.ssh && ssh-keyscan github.com >> ~/.ssh/known_hosts
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup component add rustfmt && rustup update

COPY . /solana
WORKDIR /solana
RUN --mount=type=ssh cargo build --release
RUN rm /solana/target/release/deps -rf
RUN rm /solana/target/release/build -rf


FROM ubuntu:20.04 as dest
RUN apt-get -y update && apt-get -y install libssl-dev libudev-dev curl

COPY --from=builder /solana/target/release/ /usr/local/solana
COPY ./entrypoint.sh /entrypoint.sh
ENV PATH="/usr/local/solana:$PATH"

# CMD /bin/bash
