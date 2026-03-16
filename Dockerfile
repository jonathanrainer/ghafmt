ARG GHAFMT_BUILD_METADATA=""

FROM messense/rust-musl-cross:x86_64-musl AS builder

ARG GHAFMT_BUILD_METADATA

RUN apt-get update && apt-get install -y --no-install-recommends libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .

RUN GHAFMT_BUILD_METADATA=${GHAFMT_BUILD_METADATA} cargo build --release --target x86_64-unknown-linux-musl

FROM scratch
COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/ghafmt /ghafmt
ENTRYPOINT ["/ghafmt"]
