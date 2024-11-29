let token = "";
let centrifuge;
let subOnlineStatus;
let subFactoryResetStatus;
let subSystemInfo;
let subTimeout;
let subNetworkStatus;
let subFactoryResetKeys;

document.getElementById("login").addEventListener("submit", getLoginToken);
document
	.getElementById("factory-reset")
	.addEventListener("click", showFactoryResetDialog);
document.getElementById("reboot").addEventListener("click", reboot);
document
	.getElementById("reload-network")
	.addEventListener("click", reloadNetwork);
document.getElementById("logout").addEventListener("click", logout);

const online = document.getElementById("online");
const osversion = document.getElementById("osversion");
const osname = document.getElementById("osname");
const waitOnlineTimeout = document.getElementById("wait-online-timeout");
const omnectDeviceServiceVersion = document.getElementById(
	"omnect-device-service-version",
);
const azureSdkVersion = document.getElementById("azure-sdk-version");
const bootTime = document.getElementById("boot-time");
const factoryResetStatus = document.getElementById("factory-reset-status");
const networkStatus = document.getElementById("network-status");
const dialog = document.getElementById("spinner");
const dialogTitle = document.querySelector(".dialog-title");
const spinnerTitle = document.querySelector("#spinner .dialog-title");
const factoryResetDialog = document.getElementById("factory-reset-keys");
const factoryResetKeyList = document.getElementById("factory-reset-key-list");
const alertDialog = document.getElementById("alert");
const loginErrorP = document.querySelector(".login-error");
let factoryResetKeys = ["wifi"];

factoryResetDialog.addEventListener("submit", (e) => {
	const keep = [];
	for (const k of factoryResetKeys) {
		const elem = document.querySelector(`input[name="${k}"]`);
		if (elem.checked) {
			keep.push(k);
			elem.checked = false;
		}
	}
	factoryReset(keep);
});

function bytesToBase64(bytes) {
	const binString = Array.from(bytes, (byte) =>
		String.fromCodePoint(byte),
	).join("");
	return btoa(binString);
}

async function logout() {
	centrifuge.disconnect();
	document.getElementById("login").style.display = "flex";
	document.getElementById("logout").style.display = "none";
	document.getElementById("stats").style.display = "none";
	networkStatus.innerHTML = "N/A";
}

async function getLoginToken(e) {
	e.preventDefault();
	const user = document.getElementById("user").value;
	const password = document.getElementById("password").value;
	const creds = bytesToBase64(new TextEncoder().encode(`${user}:${password}`));

	const res = await fetch("token/login", {
		method: "POST",
		headers: {
			Authorization: `Basic ${creds}`,
		},
	});

	if (!res.ok) {
		if (res.status === 401)
			loginErrorP.innerText = "Username and/or password incorrect";
		else loginErrorP.innerText = "An error occurred while checking credentials";
		return;
	}

	loginErrorP.innerText = "";
	token = await res.text();

	const centrifuge_url = `wss://${window.location.hostname}:8000/connection/websocket`;

	centrifuge = new Centrifuge(centrifuge_url, {
		token: token,
		getToken: getConnectionToken(),
	});

	centrifuge
		.on("connecting", (ctx) => {
			console.log(`connecting: ${ctx.code}, ${ctx.reason}`);
		})
		.on("connected", (ctx) => {
			console.log(`connected over ${ctx.transport}`);
			document.getElementById("login").style.display = "none";
			document.getElementById("logout").style.display = "inline-block";
			document.getElementById("stats").style.display = "block";
			document.getElementById("password").value = "";
			dialog.close();
		})
		.on("disconnected", (ctx) => {
			console.log(`disconnected: ${ctx.code}, ${ctx.reason}`);
			logout();
		})
		.connect();

	centrifuge.history("OnlineStatus", { limit: 1 }).then((resp) => {
		console.log(resp);
		if (0 < resp.publications.length) {
			setOnlineStatus(resp.publications[0].data);
		}
	});

	centrifuge.history("FactoryResetStatus", { limit: 1 }).then((resp) => {
		console.log(resp);
		if (0 < resp.publications.length) {
			setFactoryResetStatus(resp.publications[0].data);
		}
	});

	centrifuge.history("SystemInfo", { limit: 1 }).then((resp) => {
		console.log(resp);
		if (0 < resp.publications.length) {
			setSystemInfo(resp.publications[0].data);
		}
	});

	centrifuge.history("Timeouts", { limit: 1 }).then((resp) => {
		console.log(resp);
		if (0 < resp.publications.length) {
			setTimeout(resp.publications[0].data);
		}
	});

	centrifuge.history("NetworkStatus", { limit: 1 }).then((resp) => {
		console.log(resp);
		if (0 < resp.publications.length) {
			setNetworkStatus(resp.publications[0].data);
		}
	});

	centrifuge.history("FactoryResetKeys", { limit: 1 }).then((resp) => {
		console.log(resp);
		if (0 < resp.publications.length) {
			setFactoryResetKeys(resp.publications[0].data);
		}
	});

	subOnlineStatus = centrifuge.newSubscription("OnlineStatus");
	subFactoryResetStatus = centrifuge.newSubscription("FactoryResetStatus");
	subSystemInfo = centrifuge.newSubscription("SystemInfo");
	subTimeout = centrifuge.newSubscription("Timeouts");
	subNetworkStatus = centrifuge.newSubscription("NetworkStatus");
	subFactoryResetKeys = centrifuge.newSubscription("FactoryResetKeys");

	subOnlineStatus
		.on("publication", (ctx) => {
			setOnlineStatus(ctx.data);
		})
		.subscribe();

	subFactoryResetStatus
		.on("publication", (ctx) => {
			setFactoryResetStatus(ctx.data);
		})
		.subscribe();

	subSystemInfo
		.on("publication", (ctx) => {
			setSystemInfo(ctx.data);
		})
		.subscribe();

	subTimeout
		.on("publication", (ctx) => {
			setTimeout(ctx.data);
		})
		.subscribe();

	subNetworkStatus
		.on("publication", (ctx) => {
			setNetworkStatus(ctx.data);
		})
		.subscribe();

	subFactoryResetKeys
		.on("publication", (ctx) => {
			setFactoryResetKeys(ctx.data);
		})
		.subscribe();
}

async function getConnectionToken() {
	const res = await fetch("token/refresh", {
		headers: {
			Authorization: `Bearer ${token}`,
		},
	});

	token = await res.text();
	return token;
}

function setSystemInfo(data) {
	if (typeof data.os !== "undefined") {
		osversion.innerHTML = data.os.version;
		osname.innerHTML = data.os.name;
	}
	if (typeof data.omnect_device_service_version !== "undefined") {
		omnectDeviceServiceVersion.innerHTML = data.omnect_device_service_version;
	}
	if (typeof data.azure_sdk_version !== "undefined") {
		azureSdkVersion.innerHTML = data.azure_sdk_version;
	}
	if (typeof data.boot_time !== "undefined") {
		bootTime.innerHTML = new Date(data.boot_time).toLocaleString();
	}
}

function setOnlineStatus(data) {
	if (typeof data.iothub !== "undefined") {
		online.innerHTML = data.iothub;
	}
}

function setTimeout(data) {
	if (typeof data.wait_online_timeout !== "undefined") {
		waitOnlineTimeout.innerHTML = `${data.wait_online_timeout.secs}secs`;
	}
}

function setFactoryResetStatus(data) {
	if (typeof data.factory_reset_status !== "undefined") {
		factoryResetStatus.innerHTML = data.factory_reset_status;
	}
}

function setFactoryResetKeys(data) {
	if (typeof data.keys !== "undefined") {
		factoryResetKeys = data.keys;
	}
}

function setNetworkStatus(data) {
	if (typeof data.network_status !== "undefined") {
		networkStatus.innerHTML = "";
		for (const networkInterface of data.network_status) {
			const stateElement = document.createElement("div");
			stateElement.classList.add(
				networkInterface.online ? "online" : "offline",
			);

			networkStatus.appendChild(
				networkStatusAddValue(networkInterface.name, stateElement),
			);
			networkStatus.appendChild(
				networkStatusAddValue("MAC", networkInterface.mac),
			);
			if (Object.hasOwn(networkInterface, "ipv4")) {
				networkStatus.appendChild(networkStatusAddValue("IPv4"));
				for (const [i, addr] of networkInterface.ipv4.addrs.entries()) {
					const dhcpStatic = addr.dhcp ? "DHCP" : "Static";
					networkStatus.appendChild(
						networkStatusAddValue(
							`Addr${i}`,
							`${addr.addr}/${addr.prefix_len} (${dhcpStatic})`,
						),
					);
				}
				networkStatus.appendChild(
					networkStatusAddValue("DNS", networkInterface.ipv4.dns),
				);
				networkStatus.appendChild(
					networkStatusAddValue("Gateways", networkInterface.ipv4.gateways),
				);
			}

			if (Object.hasOwn(networkInterface, "ipv6")) {
				networkStatus.appendChild(networkStatusAddValue("IPv6"));
				for (const [i, addr] of networkInterface.ipv6.addrs.entries()) {
					const dhcpStatic = addr.dhcp ? "DHCP" : "Static";
					networkStatus.appendChild(
						networkStatusAddValue(
							`Addr${i}`,
							`${addr.addr}/${addr.prefix_len} (${dhcpStatic})`,
						),
					);
				}
				networkStatus.appendChild(
					networkStatusAddValue("DNS", networkInterface.ipv6.dns),
				);
				networkStatus.appendChild(
					networkStatusAddValue("Gateways", networkInterface.ipv6.gateways),
				);
			}

			const spacer = document.createElement("div");
			spacer.classList.add("spacer");

			networkStatus.appendChild(spacer);
		}
	}
}

function networkStatusAddValue(key, value) {
	const keyValueWrapper = document.createElement("div");
	keyValueWrapper.classList.add("key-value-wrapper");
	keyValueWrapper.classList.add("divider");

	const keyElem = document.createElement("div");
	keyElem.classList.add("key");
	keyElem.textContent = `${key}:`;

	const valueElem = document.createElement("div");
	if (value?.nodeType) {
		valueElem.appendChild(value);
	} else if (value) {
		valueElem.textContent = value;
	}

	keyValueWrapper.appendChild(keyElem);
	keyValueWrapper.appendChild(valueElem);

	return keyValueWrapper;
}

function networkStatusAddValues(key, value) {
	const keyValueWrapper = document.createElement("div");
	keyValueWrapper.classList.add("key-value-wrapper");

	const keyElem = document.createElement("div");
	keyElem.classList.add("key");
	keyElem.textContent = `${key}:`;

	const valueElem = document.createElement("div");
	valueElem.textContent = value.join(",");

	keyValueWrapper.appendChild(keyElem);
	keyValueWrapper.appendChild(valueElem);

	return keyValueWrapper;
}

function showFactoryResetDialog() {
	createFactoryResetKeyList();
	factoryResetDialog.showModal();
}

async function factoryReset(keep) {
	const res = await fetch("factory-reset", {
		method: "POST",
		headers: {
			Authorization: `Bearer ${token}`,
			"Content-Type": "application/json",
		},
		body: JSON.stringify({ preserve: keep }),
	});

	if (res.ok) {
		dialog.showModal();
		spinnerTitle.textContent = "Please wait. Device is resetting.";
	} else {
		alertDialog.querySelector(".dialog-title").innerText =
			"Factory reset failed";
		alertDialog.querySelector(".dialog-content").innerText =
			`Factory reset failed with status ${res.status}`;
		alertDialog.showModal();
	}
}

function createFactoryResetKeyList() {
	factoryResetKeyList.innerHTML = "";
	for (const k of factoryResetKeys) {
		const container = document.createElement("div");
		const keyElem = document.createElement("input");
		keyElem.setAttribute("type", "checkbox");
		keyElem.setAttribute("name", k);
		keyElem.setAttribute("value", k);
		keyElem.setAttribute("id", k);

		const label = document.createElement("label");
		label.setAttribute("for", k);
		label.innerText = k;

		container.appendChild(keyElem);
		container.appendChild(label);

		factoryResetKeyList.appendChild(container);
	}
}

async function reboot() {
	const res = await fetch("reboot", {
		method: "POST",
		headers: {
			Authorization: `Bearer ${token}`,
			"Content-Type": "application/json",
		},
	});

	if (res.ok) {
		dialog.showModal();
		spinnerTitle.textContent = "Please wait. Device is rebooting.";
	} else {
		alertDialog.querySelector(".dialog-title").innerText = "Reboot failed";
		alertDialog.querySelector(".dialog-content").innerText =
			`Reboot failed with status ${res.status}`;
		alertDialog.showModal();
	}
}

async function reloadNetwork() {
	const res = await fetch("reload-network", {
		method: "POST",
		headers: {
			Authorization: `Bearer ${token}`,
			"Content-Type": "application/json",
		},
	});

	if (!res.ok) {
		alertDialog.querySelector(".dialog-title").innerText =
			"Reload network failed";
		alertDialog.querySelector(".dialog-content").innerText =
			`Reload network failed with status ${res.status}`;
		alertDialog.showModal();
	}
}

const closeDialogBtn = document.querySelectorAll(".close-dialog");
for (const x of closeDialogBtn) {
	x.addEventListener("click", (e) => {
		closeDialog(e.target);
	});
}

const preventEsc = document.querySelectorAll(".prevent-esc");
for (const x of preventEsc) {
	x.addEventListener("keydown", (e) => {
		if (e.key === "Escape") {
			e.preventDefault();
		}
	});
}

function closeDialog(elem) {
	if (elem?.parentElement && elem?.parentElement.localName === "dialog") {
		elem?.parentElement.close();
	} else if (elem?.parentElement) {
		closeDialog(elem?.parentElement);
	}
}
