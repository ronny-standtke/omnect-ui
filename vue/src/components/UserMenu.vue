<script lang="ts" setup>
import { ref } from "vue"
import { useRouter } from "vue-router"
import { useCentrifuge } from "../composables/useCentrifugo"
import Menu from "./Menu.vue"

const { unsubscribeAll, disconnect } = useCentrifuge()
const router = useRouter()

const menu = ref(false)

const logOut = async () => {
	await fetch("logout", {
		method: "POST"
	})
	unsubscribeAll()
	disconnect()
	router.push("/login")
}
</script>

<template>
	<Menu v-model="menu" :open-on-hover="false">
		<template v-slot:activator="{ props }">
			<picture data-cy="user-menu" class="h-8 w-8 " v-bind="props">
				<source id="s1"
					:srcset="`https://ui-avatars.com/api/?name=ui&background=0D8ABC&color=fff&rounded=true`" />
				<img class="h-8 w-8 rounded-full cursor-pointer"
					:src="`https://ui-avatars.com/api/?name=ui&background=0D8ABC&color=fff&rounded=true`" alt=""
					onerror="this.onerror=null;document.getElementById('s1').srcset=this.src;" />
			</picture>
		</template>

		<v-card title="omnect-ui">
			<v-card-text class="mt-2">
				<div class="flex justify-space-between items-center">
					<v-btn type="button" text="Change password" prepend-icon="mdi-lock-outline" variant="text"
						color="default" @click="$router.push('/update-password')">
					</v-btn>
					<v-btn type="button" text="logout" prepend-icon="mdi-logout" variant="text" color="default"
						@click="logOut">
					</v-btn>
				</div>
			</v-card-text>
		</v-card>
	</Menu>
</template>
