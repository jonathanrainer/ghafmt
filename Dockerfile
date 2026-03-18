ARG TARGETARCH

FROM --platform=$BUILDPLATFORM ghcr.io/rust-cross/rust-musl-cross:${TARGETARCH}-musl AS builder

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
