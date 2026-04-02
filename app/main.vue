
<script use="components/text-tag.vue">
new Vue({
	el: '#app',
	data: {
		message: 'Hello Vue!',
	},
});
</script>

<div id="app">
	<h1>{{ message }}<text-tag text="tag" /></h1>
	<p>This is a simple Vue.js project that demonstrates how to use Vue components and styles.</p>
</div>
