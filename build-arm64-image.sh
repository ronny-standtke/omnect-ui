# file used for local development

# local arm64 build
omnect_ui_version=$(toml get --raw Cargo.toml package.version)

docker buildx build \
  --build-arg=DOCKER_NAMESPACE=omnectweucopsacr.azurecr.io \
  --build-arg=VERSION_RUST_CONTAINER=1.84.1-bookworm \
  --platform linux/arm64 \
  --load \
  -f Dockerfile . \
  -t omnectshareddevacr.azurecr.io/omnect-ui:$(whoami)
