<link rel="component" href="components/text-tag.vue">
<link rel="component" href="components/example-app.vue">

<script>
const app = Vue.createApp({
	data() {
		return {
			message: 'Hello Vue 3!',
		};
	},
});

app.component('example-app', ExampleApp);
app.component('text-tag', TextTag);
app.mount('#app');
</script>

<div id="app">
	<header class="app-header">
		<h1>{{ message }} <text-tag text="components" /></h1>
		<p>A small Vue 3 example showing props, events, and local component scopes.</p>
	</header>

	<example-app></example-app>
</div>

<style>
#app {
	max-width: 720px;
	margin: 32px auto;
	padding: 0 16px 32px;
	min-height: 100vh;
	font-family: sans-serif;
	line-height: 1.4;
}

.app-header {
	margin-bottom: 24px;
}

.app-header h1 {
	margin-bottom: 8px;
}

.app-header p {
	margin: 0;
}
</style>
