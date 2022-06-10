# gcc 8.2.0 x86_64-elf cross-compiler container

FROM debian:bullseye
LABEL maintainer "Brett Vickers <github.com/beevik>"

ARG BINUTILS_VERSION=2.38
ARG GCC_VERSION=12.1.0

# Install cross-compiler prerequisites
RUN set -x \
	&& apt-get update \
        && apt-get upgrade -y \
	&& apt-get install -y wget gcc libgmp3-dev libmpfr-dev libisl-dev \
		libmpc-dev texinfo bison flex make bzip2 patch \
		build-essential nasm xorriso grub-pc-bin grub-common

# Pull binutils and gcc source code
RUN set -x \
	&& mkdir -p /usr/local/src \
	&& cd /usr/local/src \
	&& wget -q https://ftp.gnu.org/gnu/binutils/binutils-${BINUTILS_VERSION}.tar.gz \
	&& wget -q https://ftp.gnu.org/gnu/gcc/gcc-${GCC_VERSION}/gcc-${GCC_VERSION}.tar.gz \
	&& tar zxf binutils-${BINUTILS_VERSION}.tar.gz \
	&& tar zxf gcc-${GCC_VERSION}.tar.gz \
	&& rm binutils-${BINUTILS_VERSION}.tar.gz gcc-${GCC_VERSION}.tar.gz \
	&& chown -R root:root binutils-${BINUTILS_VERSION} \
	&& chown -R root:root gcc-${GCC_VERSION} \
	&& chmod -R o-w,g+w binutils-${BINUTILS_VERSION} \
	&& chmod -R o-w,g+w gcc-${GCC_VERSION}

# Copy compile scripts
COPY src /usr/local/src/

# Build and install binutils and the cross-compiler
RUN set -x \
	&& cd /usr/local/src \
	&& ./build-binutils.sh ${BINUTILS_VERSION} \
	&& ./build-gcc.sh ${GCC_VERSION}

CMD ["/bin/bash"]
