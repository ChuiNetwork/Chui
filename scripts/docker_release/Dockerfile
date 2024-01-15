FROM ubuntu:20.04
ARG release_version=LATEST_CHUI_RELEASE
RUN apt-get -y update && apt-get -y install libssl-dev libudev-dev curl
RUN curl -sSf https://raw.githubusercontent.com/chui/Chui/develop/install/chui-install-init.sh | sh -s - LATEST_CHUI_RELEASE
ENV PATH="/root/.local/share/chui/install/active_release/bin":"$PATH"
COPY entrypoint.sh entrypoint.sh
ENTRYPOINT [ "/usr/bin/sh","-c", "./entrypoint.sh" ]
