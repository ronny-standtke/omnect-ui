# omnect-ui
**Product page: https://www.omnect.io/home**

This module implements a web frontend and backend to provide omnect specific features in a local environment, where the device might not be connected to the azure cloud. In that case the device cannot be remotely controlled by [omnect-portal](https://cp.omnect.conplement.cloud/) and omnect-ui might be the alternative.

## Install omnect-ui

Since omnect-os is designed as generic OS, all specific or optional applications must be provided as docker images. There are two options to install omnect-ui on a target:
1. azure iotedge deployment:
   - deployment of omnect-ui docker image via omnect-portal to a device in field
   - device must be online (at least once) in order to receive the deployment
   - after a factory reset omnect-ui must be deployed again what requires a connection to azure cloud
2. in-factory installation (check [meta-omnect](https://github.com/omnect/meta-omnect) for partition layout):
   - inject omnect-ui docker image into factory partition
   - omnect-os takes care of installation while first boot and after factory reset

### iotedge deployment

In case it is agreed the omnect team takes care of providing omnect-ui as application in omnect-portal. Get into contact with support@omnect.io if interested. 

### Inject into omnect-os image

If omnect-ui must be part of the omnect-os image, several configuration files have to be injected into an omnect-os firmware image:
1.  in all files in config/ folder replace all occurrences of %% *** %% with reasonable values:
    1.  %%CENTRIFUGO_API_KEY%%: the [API key](https://centrifugal.dev/docs/server/server_api#http-api) should come from a vault
    2.  %%CENTRIFUGO_TOKEN_HMAC_SECRET_KEY%%: the [HMAC key](https://centrifugal.dev/docs/server/authentication) should come from a vault
    3.  %%USER%%: user name to be matched on omnect-ui login
    4.  %PASSWORD%%: password to be matched on omnect-ui login
2.  it might be appropriate to adapt other default config values to your needs
3.  inject config files via [omnect-cli](https://github.com/omnect/omnect-cli) into omnect-os image
```
# download and copy omnect-ui docker image
omnect-cli docker inject -d omnectsharedprodacr.azurecr.io/omnect-portal-omnect-ui:latest -e /oci_images/omnect-ui.tar.gz -i my-omnect-os-image.wic

# copy config files
omnect-cli file copy-to-image \
	-f omnect-device-service.env,factory:/etc/omnect/omnect-device-service.env \
	-f omnect-ui.env,factory:/etc/omnect/omnect-ui.env \
	-f publish_endpoints.json,factory:/etc/omnect/publish_endpoints.json \
	-f omnect-ui.service,factory:/etc/systemd/system/omnect-ui.service \
	-f create-fs-links.txt,factory:/etc/omnect/create-fs-links.txt \
	-i my-omnect-os-image.wic

# copy certificates 
# only in case not already done (e.g. device provisioned by tpm)
# devices provisioned by x509 usually already have certs injected
omnect-cli identity set-device-certificate \
  -d "my-device-id" \
  -c my-omnect-int-ca-fullchain.pem \
  -k my-omnect-int-ca.key -D 365 \
  -i my-omnect-os-image.wic
```

## Access omnect-ui

omnect-ui can be reached at https://DeviceHostnameOrIp:1977<br>

Login with the configured credentials<br>
![login](docu/login.png)<br>
Watch device status<br>
![login](docu/main.png)

# License

Licensed under either of
* Apache License, Version 2.0, (./LICENSE-APACHE or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license (./LICENSE-MIT or <http://opensource.org/licenses/MIT>)

at your option.

# Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

---

copyright (c) 2024 conplement AG<br>
Content published under the Apache License Version 2.0 or MIT license, are marked as such. They may be used in accordance with the stated license conditions.
