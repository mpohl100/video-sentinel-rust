#Use microsoft devcontainer
ARG VARIANT=trixie
FROM mcr.microsoft.com/vscode/devcontainers/rust:${VARIANT}

RUN apt update \
	&& apt install -y --no-install-recommends \
		cmake \
		linux-perf \
		pkg-config \
		clang \
		libclang-dev \
		libopencv-dev \
		libavutil-dev \
		libavformat-dev \
		libavcodec-dev \
		libavfilter-dev \
		libavdevice-dev \
		libswscale-dev \
		libswresample-dev \
	&& rm -rf /var/lib/apt/lists/*

RUN cargo install flamegraph
