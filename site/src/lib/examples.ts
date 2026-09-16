export type PlaygroundExample = {
	id: string;
	label: string;
	source: string;
	args?: Record<string, unknown> | null;
};

export const PLAYGROUND_EXAMPLES: PlaygroundExample[] = [
	{
		id: 'hello',
		label: 'Hello world',
		source: `import core from "grapheme/core"

query HelloWorld {
  set { message: "LETS GO?!!!!!" }
  |> core.echo(message: $state.message)
}
`
	},
	{
		id: 'merge',
		label: 'Core merge',
		source: `import core from "grapheme/core"

query CoreMergeDemo {
  set { language: "grapheme", version: "0.7", mode: "dev" }
  |> core.merge(
    left: $state,
    right: { mode: "runtime-wasm", playground: true }
  )
}
`
	},
	{
		id: 'params',
		label: 'Params + call',
		source: `import core from "grapheme/core"

query ParamsCallBind($label: String = "world") {
  call Greet(label: $label)
}

iterator Greet($label: String) on Any {
  core.echo(message: "hello {$label}")
}
`,
		args: { label: 'grapheme' }
	},
	{
		id: 'json',
		label: 'JSON transform',
		source: `import core from "grapheme/core"
import json from "grapheme/json"

query JsonRoundtrip {
  set {
    payload: { team: "ops", score: 98, flags: ["canary", "ok"] }
  }
  |> json.stringify(value: $state.payload)
  |> json.parse(text: $state.text)
  |> core.echo(message: "parsed score={$state.value.score}")
}
`
	}
];

export const DEFAULT_EXAMPLE = PLAYGROUND_EXAMPLES[0]!;
