# Copyright (c), Mysten Labs, Inc.
# SPDX-License-Identifier: Apache-2.0

# This Dockerfile references StageX reproducible containers, which provide
# deterministic, versioned builds for essential toolchains and libraries like
# Rust, GCC, LLVM, OpenSSL, and more. It also includes custom containers
# (mysten/python-repro and mysten/socat-repro) based on StageX to support
# VSOCK communication in secure and isolated build environments.

FROM stagex/binutils:sx2024.09.0@sha256:30a1bd110273894fe91c3a4a2103894f53eaac43cf12a035008a6982cb0e6908 AS binutils
FROM stagex/ca-certificates:sx2024.11.0@sha256:a84695f983a448a82acfe78af11f33c6a66b27124266e1fdc3ecfb8dc5852573 AS ca-certificates
FROM stagex/gcc:sx2024.09.0@sha256:439bf36289ef036a934129d69dd6b4c196427e4f8e28bc1a3de5b9aab6e062f0 AS gcc
FROM stagex/zlib:sx2024.09.0@sha256:96b4100550760026065dac57148d99e20a03d17e5ee20d6b32cbacd61125dbb6 AS zlib
FROM stagex/llvm:sx2024.09.0@sha256:30517a41af648305afe6398af5b8c527d25545037df9d977018c657ba1b1708f AS llvm
FROM stagex/openssl:sx2024.09.0@sha256:2c1a9d8fcc6f52cb11a206f380b17d74c1079f04cbb08071a4176648b4df52c1 AS openssl
FROM stagex/eif_build:sx2024.09.0@sha256:291653f1ca528af48fd05858749c443300f6b24d2ffefa7f5a3a06c27c774566 AS eif_build
FROM stagex/gen_initramfs:sx2024.09.0@sha256:f5b9271cca6003e952cbbb9ef041ffa92ba328894f563d1d77942e6b5cdeac1a AS gen_initramfs
FROM stagex/libunwind:sx2024.09.0@sha256:97ee6068a8e8c9f1c74409f80681069c8051abb31f9559dedf0d0d562d3bfc82 AS libunwind
FROM stagex/rust:sx2024.09.0@sha256:b7c834268a81bfcc473246995c55b47fe18414cc553e3293b6294fde4e579163 AS rust
FROM stagex/musl:sx2024.09.0@sha256:ad351b875f26294562d21740a3ee51c23609f15e6f9f0310e0994179c4231e1d AS musl
FROM stagex/git:sx2024.09.0@sha256:29a02c423a4b55fa72cf2fce89f3bbabd1defea86d251bb2aea84c056340ab22 AS git
FROM stagex/pkgconf:sx2024.09.0@sha256:ba7fce4108b721e8bf1a0d993a5f9be9b65eceda8ba073fe7e8ebca2a31b1494 AS pkgconf
FROM stagex/busybox:sx2024.09.0@sha256:d34bfa56566aa72d605d6cbdc154de8330cf426cfea1bc4ba8013abcac594395 AS busybox
FROM stagex/linux-nitro:sx2024.03.0@sha256:073c4603686e3bdc0ed6755fee3203f6f6f1512e0ded09eaea8866b002b04264 AS linux-nitro
FROM stagex/cpio:sx2024.11.0@sha256:8af5412a6c0cf20cdb70896ea5a2bda1f6c36d9411d3c28bcee8e42cdc7bd5db AS cpio
FROM mysten/socat-repro:latest@sha256:9fd1f1bfd15544047446486f5f9291737d097ee10d0db2c176d5b4f967120917 as socat
FROM stagex/jq:sx2024.11.0@sha256:f54ab8399ca0b373d34a61e2aadd0bb28fac54841c9495043fd477316ceefd7c AS jq
FROM mysten/python-repro:latest@sha256:489aed0536eaf27b06039ea7df8a7b6a0907aa079d8d06d8f6b31533826a3a12 as python
FROM scratch as base
ENV TARGET=x86_64-unknown-linux-musl
ENV RUSTFLAGS="-C target-feature=+crt-static"
ENV CARGOFLAGS="--locked --no-default-features --release --target ${TARGET}"
ENV OPENSSL_STATIC=true

COPY --from=busybox . /
COPY --from=musl . /
COPY --from=libunwind . /
COPY --from=openssl . /
COPY --from=zlib . /
COPY --from=ca-certificates . /
COPY --from=binutils . /
COPY --from=pkgconf . /
COPY --from=git . /
COPY --from=rust . /
COPY --from=gen_initramfs . /
COPY --from=eif_build . /
COPY --from=llvm . /
COPY --from=gcc . /
COPY --from=cpio . /
COPY --from=linux-nitro /bzImage .
COPY --from=linux-nitro /nsm.ko .
COPY --from=linux-nitro /linux.config .

#ADD . /src
FROM base as build

# Copy source code
COPY . /src/
COPY Cargo.toml . 
COPY Cargo.lock .


# Build init,aws,system as a workspace.
WORKDIR /src
RUN cargo build --workspace --locked --no-default-features --release --target x86_64-unknown-linux-musl

# Build nautilus server
WORKDIR /src/src/nautilus-server
RUN ls -al
ENV RUSTFLAGS="-C target-feature=+crt-static -C relocation-model=static"
RUN cargo build --locked --no-default-features --release --target x86_64-unknown-linux-musl

# Setup cpio build environment
WORKDIR /build_cpio
ENV KBUILD_BUILD_TIMESTAMP=1

RUN mkdir -p /initramfs_files
RUN mkdir -p initramfs_files/usr/local/bin
COPY --from=linux-nitro /nsm.ko initramfs_files/nsm.ko
COPY --from=busybox . initramfs_files
COPY --from=musl . initramfs_files
COPY --from=python . initramfs_files
RUN chown root:root initramfs_files/bin/python3

RUN cp /src/target/${TARGET}/release/init initramfs_files
RUN ls -al /src/src
RUN ls -al /src/src/nautilus-server

RUN cp /src/src/nautilus-server/target/${TARGET}/release/nautilus-server initramfs_files

RUN cp /src/src/nautilus-server/traffic_forwarder.py initramfs_files/
RUN cp /src/src/nautilus-server/run.sh initramfs_files/
RUN cp /src/src/nautilus-server/allowed_endpoints.yaml initramfs_files/

COPY --from=ca-certificates /etc/ssl/certs initramfs_files
COPY --from=busybox /bin/sh initramfs_files/sh
COPY --from=jq /bin/jq initramfs_files
# Socat is statically linked and custom flags so no requirements of lib
COPY --from=socat . initramfs_files

RUN <<-EOF
    set -eux
    cd initramfs_files
    find . -exec touch -hcd "@0" "{}" +
    find . -print0 \
    | sort -z \
    | cpio \
        --null \
        --create \
        --verbose \
        --reproducible \
        --format=newc \
    | gzip --best \
    > /build_cpio/rootfs.cpio
EOF

WORKDIR /build_eif
RUN eif_build \
	--kernel /bzImage \
	--kernel_config /linux.config \
	--ramdisk /build_cpio/rootfs.cpio \
	--pcrs_output /nitro.pcrs \
	--output /nitro.eif \
	--cmdline 'reboot=k initrd=0x2000000,3228672 root=/dev/ram0 panic=1 pci=off nomodules console=ttyS0 i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd'

FROM base as install
WORKDIR /rootfs
COPY --from=build /nitro.eif .
COPY --from=build /nitro.pcrs .
COPY --from=build /build_cpio/rootfs.cpio .
FROM scratch as package
COPY --from=install /rootfs .