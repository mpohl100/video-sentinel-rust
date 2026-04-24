#Use microsoft devcontainer
ARG VARIANT=bullseye
FROM mcr.microsoft.com/vscode/devcontainers/rust:${VARIANT}

RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
		pkg-config \
		clang \
		libclang-dev \
		libavutil-dev \
		libavformat-dev \
		libavcodec-dev \
		libavfilter-dev \
		libavdevice-dev \
		libswscale-dev \
		libswresample-dev \
	&& rm -rf /var/lib/apt/lists/*
