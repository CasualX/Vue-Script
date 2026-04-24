<link rel="component" href="example-app/counter-panel.vue">
<link rel="component" href="example-app/entry-table.vue">

<script>
const ExampleApp = {
	template: '#example-app',
	components: {
		CounterPanel,
		EntryTable,
	},
	data() {
		return {
			count: 0,
			entries: [
				{ id: 1, name: 'Ava', role: 'Designer' },
				{ id: 2, name: 'Milo', role: 'Developer' },
			],
		};
	},
	methods: {
		increment() {
			this.count += 1;
		},
		decrement() {
			if (this.count > 0) {
				this.count -= 1;
			}
		},
		addEntry(entry) {
			this.entries.push({
				id: this.entries.length + 1,
				name: entry.name,
				role: entry.role,
			});
		},
	},
};
</script>

<template id="example-app">
	<section class="example-app">
		<counter-panel
			:count="count"
			@increment="increment"
			@decrement="decrement"
		></counter-panel>

		<entry-table :entries="entries" @add-entry="addEntry"></entry-table>
	</section>
</template>

<style>
.example-app {
	display: grid;
	gap: 24px;
}
</style>
