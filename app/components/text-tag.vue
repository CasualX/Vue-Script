<link rel="component" href="script.vue.js">

<script>
const TextTag = {
	template: '#text-tag',
	props: {
		text: String,
	},
	computed: {
		allCaps() {
			return allCaps(this.text);
		},
	},
};
</script>

<template id="text-tag">
	<div class="text-tag">{{ allCaps }}</div>
</template>

<style>
.text-tag {
	display: inline-block;
	border: 1px solid #ccc;
	padding: 5px 10px;
	border-radius: 4px;
}
</style>
