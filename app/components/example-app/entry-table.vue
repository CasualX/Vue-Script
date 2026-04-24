<script>
const EntryTable = {
	template: '#entry-table',
	data() {
		return {
			name: '',
			role: '',
		};
	},
	props: {
		entries: Array,
	},
	emits: ['add-entry'],
	methods: {
		submit() {
			var name = this.name.trim();
			var role = this.role.trim();

			if (!name || !role) {
				return;
			}

			this.$emit('add-entry', {
				name: name,
				role: role,
			});

			this.name = '';
			this.role = '';
		},
	},
};
</script>

<template id="entry-table">
	<section class="entry-table">
		<h2>Entries</h2>
		<table>
			<thead>
				<tr>
					<th>ID</th>
					<th>Name</th>
					<th>Role</th>
				</tr>
			</thead>
			<tbody>
				<tr v-for="entry in entries" :key="entry.id">
					<td>{{ entry.id }}</td>
					<td>{{ entry.name }}</td>
					<td>{{ entry.role }}</td>
				</tr>
				<tr class="entry-table__new-row">
					<td>
						<button type="button" @click="submit">Add</button>
					</td>
					<td>
						<input
							v-model="name"
							type="text"
							placeholder="Name"
							@keydown.enter.prevent="submit"
						/>
					</td>
					<td>
						<input
							v-model="role"
							type="text"
							placeholder="Role"
							@keydown.enter.prevent="submit"
						/>
					</td>
				</tr>
			</tbody>
		</table>
	</section>
</template>

<style>
.entry-table {
	margin: 24px 0;
}

.entry-table table {
	border-collapse: collapse;
	width: 100%;
}

.entry-table th,
.entry-table td {
	padding: 6px 10px;
	border: 1px solid #ccc;
	text-align: left;
}

.entry-table input {
	width: 100%;
	padding: 0;
	border: 0;
	outline: 0;
	font: inherit;
	background: transparent;
	box-sizing: border-box;
}

.entry-table button {
	padding: 4px 10px;
	font: inherit;
}
</style>
