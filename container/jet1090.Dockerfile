FROM ubuntu:22.04
ARG VERSION

RUN apt-get update 
RUN apt-get install -y --no-install-recommends curl ca-certificates xz-utils libsoapysdr-dev 

RUN useradd -ms /bin/bash user
USER user
ENV PATH="/home/user/.cargo/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -LsSf https://github.com/xoolive/rs1090/releases/download/v$VERSION/jet1090-installer.sh | sh
RUN echo 'eval "$(jet1090 --completion bash)"' >> ~/.bashrc

CMD jet1090 --help
