# check=skip=FromPlatformFlagConstDisallowed
# --platform=linux/amd64 is intentional on both FROM lines below. All CI runners are amd64
# and we use cross-compilation (rather than native ARM64 runners) to produce the ARM64 binary.
# The rust-musl-cross images support multi-arch hosts, but we pin to amd64 to ensure the
# cross-compilation toolchain is always used consistently. Removing --platform would cause
# builds to break on any non-amd64 host that tried to pull these images natively.
ARG TARGETARCH

# Digest-pinned per-platform base images. To update, run:
#   docker buildx imagetools inspect ghcr.io/rust-cross/rust-musl-cross:<tag> --format '{{json .Manifest}}' | jq -r .digest
FROM --platform=linux/amd64 ghcr.io/rust-cross/rust-musl-cross:amd64-musl@sha256:bcf6a66615f9d5bae659e38ab4311260e0488d1c34ad0ab9f9147f4cd5ef64ed AS base-amd64
FROM --platform=linux/amd64 ghcr.io/rust-cross/rust-musl-cross:arm64-musl@sha256:eab6a58ff66eaa33fa87fc31ed11403596719ca3f23aa51626fb993d77c1200b AS base-arm64

FROM base-${TARGETARCH} AS builder

RUN apt-get update && apt-get install -y --no-install-recommends libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .

# CARGO_BUILD_TARGET and RUST_MUSL_CROSS_TARGET are set by the rust-cross image.
# BINDGEN_EXTRA_CLANG_ARGS points clang at the musl sysroot so bindgen can find headers.
# Copy the binary to a fixed path so the scratch stage doesn't need to know the target.
RUN BINDGEN_EXTRA_CLANG_ARGS="--sysroot=/usr/local/musl/${RUST_MUSL_CROSS_TARGET}" \
    cargo build --release \
    && cp "target/${RUST_MUSL_CROSS_TARGET}/release/ghafmt" /ghafmt

FROM scratch
COPY --from=builder /ghafmt /ghafmt
ENTRYPOINT ["/ghafmt"]
