FROM mcr.microsoft.com/devcontainers/base:ubuntu26.04

ENV DEBIAN_FRONTEND=noninteractive
ENV ROS_DISTRO=lyrical
ENV AMENT_PREFIX_PATH=/opt/ros/lyrical
ENV CMAKE_PREFIX_PATH=/opt/ros/lyrical
ENV COLCON_PREFIX_PATH=/opt/ros/lyrical
ENV LD_LIBRARY_PATH=/opt/ros/lyrical/lib
ENV PATH=/opt/ros/lyrical/bin:${PATH}
ENV PYTHONPATH=/opt/ros/lyrical/lib/python3.14/site-packages

RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
		curl \
		lsb-release \
		software-properties-common \
	&& add-apt-repository universe \
	&& export ROS_APT_SOURCE_VERSION=$(curl -s https://api.github.com/repos/ros-infrastructure/ros-apt-source/releases/latest | grep -F '"tag_name"' | awk -F '"' '{print $4}') \
	&& curl -L -o /tmp/ros2-apt-source.deb "https://github.com/ros-infrastructure/ros-apt-source/releases/download/${ROS_APT_SOURCE_VERSION}/ros2-apt-source_${ROS_APT_SOURCE_VERSION}.$(. /etc/os-release && echo ${UBUNTU_CODENAME:-${VERSION_CODENAME}})_all.deb" \
	&& dpkg -i /tmp/ros2-apt-source.deb \
	&& apt-get update \
	&& apt-get install -y --no-install-recommends \
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
		python3-colcon-common-extensions \
		ros-dev-tools \
		ros-lyrical-ros-base \
		ros-lyrical-sensor-msgs \
		ros-lyrical-vision-msgs \
	&& rm -f /tmp/ros2-apt-source.deb \
	&& rm -rf /var/lib/apt/lists/* \
	&& echo "source /opt/ros/lyrical/setup.bash" >> /etc/bash.bashrc
