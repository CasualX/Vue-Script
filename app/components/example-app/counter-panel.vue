<script>
const CounterPanel = {
	template: '#counter-panel',
	props: {
		count: Number,
	},
	emits: ['decrement', 'increment'],
};
</script>

<template id="counter-panel">
	<section class="counter-panel">
		<h2>Counter</h2>
		<div class="counter-panel__controls">
			<button type="button" @click="$emit('decrement')">-</button>
			<span class="counter-panel__value">{{ count }}</span>
			<button type="button" @click="$emit('increment')">+</button>
		</div>
	</section>
</template>

<style>
.counter-panel {
	margin: 24px 0;
}

.counter-panel__controls {
	display: inline-flex;
	align-items: center;
	gap: 12px;
}

.counter-panel__value {
	min-width: 32px;
	text-align: center;
	font-size: 24px;
}

.counter-panel button {
	width: 40px;
	height: 40px;
	font-size: 24px;
}
</style>
