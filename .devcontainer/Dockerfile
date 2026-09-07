#Use microsoft devcontainer
ARG VARIANT=trixie
FROM mcr.microsoft.com/vscode/devcontainers/rust:${VARIANT}

RUN apt update \
	&& apt install -y --no-install-recommends \
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
