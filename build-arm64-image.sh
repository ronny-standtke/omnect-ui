# file used for local development

# local arm64 build
omnect_ui_version=$(toml get --raw Cargo.toml workspace.package.version)

docker buildx build \
  --platform linux/arm64 \
  --load \
  -f Dockerfile . \
  -t omnectshareddevacr.azurecr.io/omnect-ui:$(whoami)
