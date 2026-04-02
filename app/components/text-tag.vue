
<script use="script.vue.js">
Vue.component('text-tag', {
	template: '#text-tag',
	data() {
		return {};
	},
	props: {
		text: String,
	},
	computed: {
		all_caps() {
			return allCaps(this.text);
		},
	},
});
</script>

<template id="text-tag">
	<div class="text-tag">{{ all_caps }}</div>
</template>

<style>
.text-tag {
	display: inline-block;
	border: 1px solid #ccc;
	padding: 5px 10px;
	border-radius: 4px;
}
</style>
