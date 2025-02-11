# local arm64 build
omnect_ui_version=$(toml get --raw Cargo.toml package.version)

docker buildx build \
  --build-arg=DOCKER_NAMESPACE=omnectweucopsacr.azurecr.io \
  --build-arg=VERSION_RUST_CONTAINER=1.84.1-bookworm \
  --output type=docker,dest=./omnect-ui.tar.gz,compression=gzip,compression-level=9,name=omnect/omnect-ui:${omnect_ui_version} \
  --platform linux/arm64 \
  -f Dockerfile .