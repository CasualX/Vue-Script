type __VueScriptMethod = (...args: any[]) => any;
type __VueScriptMethods = Record<string, __VueScriptMethod>;
type __VueScriptComputed = Record<string, __VueScriptMethod | { get: __VueScriptMethod; set?: __VueScriptMethod }>;
type __VueScriptComputedValues<C extends __VueScriptComputed> = {
	readonly [K in keyof C]: C[K] extends (...args: any[]) => infer R
		? R
		: C[K] extends { get: (...args: any[]) => infer R }
			? R
			: never;
};
type __VueScriptPropValue<T> = T extends StringConstructor
	? string
	: T extends NumberConstructor
		? number
		: T extends BooleanConstructor
			? boolean
			: T extends ArrayConstructor
				? unknown[]
				: T extends ObjectConstructor
					? Record<string, unknown>
					: T extends abstract new (...args: any[]) => infer I
						? I
						: T extends { type: infer U }
							? __VueScriptPropValue<U>
							: unknown;
type __VueScriptProps<P extends Record<string, unknown>> = {
	readonly [K in keyof P]: __VueScriptPropValue<P[K]>;
};
type __VueScriptInstance<
	D extends Record<string, unknown>,
	C extends __VueScriptComputed,
	M extends __VueScriptMethods,
	P extends Record<string, unknown>,
	E extends string,
> = D & __VueScriptComputedValues<C> & M & __VueScriptProps<P> & {
	$emit(event: E, ...args: any[]): void;
	$el: any;
	$refs: Record<string, any>;
	$nextTick(callback?: () => void): Promise<void>;
};
type __VueScriptComponentOptions<
	D extends Record<string, unknown>,
	C extends __VueScriptComputed,
	M extends __VueScriptMethods,
	P extends Record<string, unknown>,
	E extends string,
> = {
	template?: string;
	components?: Record<string, unknown>;
	props?: P;
	emits?: readonly E[] | Record<E, __VueScriptMethod>;
	data?: (this: __VueScriptProps<P>) => D;
	computed?: C & ThisType<__VueScriptInstance<D, C, M, P, E>>;
	methods?: M & ThisType<__VueScriptInstance<D, C, M, P, E>>;
	[option: string]: unknown;
} & ThisType<__VueScriptInstance<D, C, M, P, E>>;
declare function __vueScriptDefineComponent<
	D extends Record<string, unknown> = {},
	C extends __VueScriptComputed = {},
	M extends __VueScriptMethods = {},
	P extends Record<string, unknown> = {},
	E extends string = string,
>(options: __VueScriptComponentOptions<D, C, M, P, E>): unknown;
declare const Vue: {
	defineComponent: typeof __vueScriptDefineComponent;
	createApp(...args: any[]): any;
	[api: string]: any;
};
