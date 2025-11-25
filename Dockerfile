ARG DOCKER_NAMESPACE=omnectweucopsacr.azurecr.io
ARG VERSION_RUST_CONTAINER=1.89.0-bookworm
ARG DISTROLESS_IMAGE=gcr.io/distroless/base-debian12:nonroot

FROM oven/bun AS vue-install
RUN mkdir -p /tmp
COPY src/ui/package.json /tmp
COPY src/ui/bun.lock /tmp
RUN cd /tmp && bun install --frozen-lockfile

FROM oven/bun AS vue-build
WORKDIR /usr/src/app
COPY src/ui .
COPY --from=vue-install /tmp/node_modules node_modules
RUN bun run build

FROM ${DISTROLESS_IMAGE} AS distroless

FROM ${DOCKER_NAMESPACE}/rust:${VERSION_RUST_CONTAINER} AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    cmake \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

ARG TARGETARCH
ARG OMNECT_UI_BUILD_ARG=""
WORKDIR "/work"

ARG CENTRIFUGO_VERSION=v6.1.0

RUN curl -sSLf https://centrifugal.dev/install.sh | sh

COPY --from=distroless /var/lib/dpkg/status.d /distroless_pkgs

COPY Cargo.lock ./Cargo.lock
COPY Cargo.toml ./Cargo.toml
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build ${OMNECT_UI_BUILD_ARG} --release -p omnect-ui --target-dir ./build

COPY .git ./.git
RUN --mount=type=cache,target=/usr/local/cargo/registry <<EOF
  set -e
  # update timestamps to force a new build
  touch /work/src/backend/src/main.rs
  cargo build ${OMNECT_UI_BUILD_ARG} --release -p omnect-ui --target-dir ./build
EOF

SHELL ["/bin/bash", "-c"]
RUN <<EOT
    set -eu

    mkdir -p /copy/status.d

    executable=(build/release/omnect-ui)

    mkdir -p /copy/$(dirname "${executable}")
    cp "${executable}" /copy/"${executable}"

    # gather libraries installed in distroless image to skip them
    readarray -t FILTER < <(for file in $(find /distroless_pkgs -type f -! -name "*.md5sums"); do sed -n "s/Package: \(.*\)$/\1/p" $file; done)

    # skip .so of the dynamic linker
    LOADER=$(readelf -l "${executable}" | grep "interpreter:" | sed -e "s/.*interpreter: \(.*\)]$/\1/")

    readarray -t LIBS < <(ldd "${executable}" | awk '{if ($3 == "") print $1; else print $3}')

    for LIB in ${LIBS[@]}; do
        # skip the linker loader
        if [ "$LIB" == "$LOADER" ]; then
            continue
        fi

        # the actual library location in the package may deviate from what the
        # linker specifies, so update that info and gather the package name.
        PKG_INFO=$(LOCALE=C.UTF-8 dpkg -S "*$LIB" 2> /dev/null) || continue
        PKG="${PKG_INFO%%:*}"
        LIB="${PKG_INFO##*: }"

        # skip libraries already installed in distroless
        if [[ " ${FILTER[*]} " =~ "${PKG} " ]]; then
            continue
        fi

        # copy the library and its dpkg database entries
        mkdir -p /copy/$(dirname "${LIB}")
        cp "${LIB}" /copy/"${LIB}"
        sed -n "/Package: ${PKG}/,/^$/p" /var/lib/dpkg/status > "/copy/status.d/${PKG}"
    done
EOT

RUN mkdir /cert

FROM ${DISTROLESS_IMAGE} AS base
COPY --from=builder --chown=10000:10000 /cert /cert
COPY --from=builder /work/build/release/omnect-ui /
COPY --from=builder /work/centrifugo /
COPY --from=builder /copy/lib/ /lib/
COPY --from=builder /copy/status.d /var/lib/dpkg/status.d
COPY --from=vue-build /usr/src/app/dist /dist/

WORKDIR "/"
COPY src/backend/config/centrifugo_config.json /

ENTRYPOINT [ "/omnect-ui" ]
