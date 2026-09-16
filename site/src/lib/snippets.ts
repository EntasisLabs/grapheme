/**
 * Every snippet here has been executed through grapheme-wasm.wasm.
 * `output` is the real `final_state.current` from that run.
 */
export type Snippet = {
	id: string;
	label: string;
	title: string;
	blurb: string;
	source: string;
	args?: Record<string, unknown> | null;
	output: string;
	steps?: number;
	tags: string[];
};

export const SNIPPETS: Snippet[] = [
	{
		id: 'rollout',
		label: 'Typed state loop',
		title: 'A rollout that reads like a runbook',
		blurb:
			'Declare the shape of your state, loop with an explicit budget, and branch on status. No hidden control flow.',
		source: `import core from "grapheme/core"

struct Release {
  service: String
  step: Float
  status: String
}

glyph Main { Rollout }

query Rollout on Any -> Release {
  Release { service: "checkout", step: 0.0, status: "ramping" }
  |> Ramp
}

iterator Ramp on Release -> Release @core_default @loop(max: 10, merge: "replace") {
  match $state.status {
    case complete => return
    default => Advance
  }
}

mutation Advance on Release -> Release @core_default {
  inc_field(field: "step")
  |> if $state.step >= 3.0 then set { status: "complete" } else set { status: "ramping" }
}
`,
		output: `{
  "service": "checkout",
  "status": "complete",
  "step": 3
}`,
		steps: 15,
		tags: ['struct', 'glyph', 'iterator', 'mutation', '@loop', 'match']
	},
	{
		id: 'params',
		label: 'Params & call',
		title: 'Executables take parameters',
		blurb:
			'Named `$params` with defaults on any executable. Bind them at the call site, from the CLI with --args-json, or from the SDK.',
		source: `import core from "grapheme/core"

query ParamsCallBind($label: String = "world") {
  call Greet(label: $label)
}

iterator Greet($label: String) on Any {
  core.echo(message: "hello {$label}")
}
`,
		args: { label: 'grapheme' },
		output: `{
  "message": "hello grapheme"
}`,
		steps: 2,
		tags: ['$params', 'call', 'defaults']
	},
	{
		id: 'tags',
		label: 'Tags & using',
		title: 'Ambient context, explicitly scoped',
		blurb:
			'A `tag` declares ambient bindings such as auth or session. `using` activates them for one block only — read `$token` outside and the compiler stops you.',
		source: `import core from "grapheme/core"

tag auth {
  $token: String
}

query TagUsingScope {
  using auth(token: "secret") {
    core.echo(message: "token={$token}")
  }
  |> core.echo(message: "after")
}
`,
		output: `{
  "message": "after"
}`,
		steps: 2,
		tags: ['tag', 'using', 'scope']
	},
	{
		id: 'csv',
		label: 'Data pipeline',
		title: 'Shape data in the open',
		blurb:
			'Parse, filter, project. Every step is a visible transformation over `$state`, and every step lands in the execution trace.',
		source: `import core from "grapheme/core"
import csv from "grapheme/csv"

query ActivePeople {
  set {
    raw: "name,team,status\\nAda,platform,active\\nLin,data,inactive\\nSam,platform,active"
  }
  |> csv.to_list(text: $state.raw)
  |> core.filter(items: $state, field: "status", equals: "active")
  |> core.map(items: $state, field: "name")
}
`,
		output: `[
  "Ada",
  "Sam"
]`,
		steps: 4,
		tags: ['csv', 'core.filter', 'core.map']
	},
	{
		id: 'totals',
		label: 'Aggregate',
		title: 'Reduce without a for-loop',
		blurb: 'Project a field, then fold it. Built-in reducers: sum, min, max, avg, count, concat, first, last.',
		source: `import core from "grapheme/core"

query Totals {
  set {
    orders: [
      { sku: "a", amount: 12.5 },
      { sku: "b", amount: 7.25 },
      { sku: "c", amount: 30.0 }
    ]
  }
  |> core.map(items: $state.orders, field: "amount")
  |> core.reduce(items: $state, mode: "sum")
}
`,
		output: `49.75`,
		steps: 3,
		tags: ['core.map', 'core.reduce']
	},
	{
		id: 'validate',
		label: 'Validate & branch',
		title: 'Guard, then transition',
		blurb:
			'Schema checks return structured results. Branch on them and transition state with an audit note — the same shape every run.',
		source: `import core from "grapheme/core"

query CoreValidateSchemaDemo {
  set { candidate: { name: "Ada", role: "Engineer" } }
  |> ValidateCandidate
}

iterator ValidateCandidate on Any {
  core.validate_schema(
    required: ["name", "role", "team"],
    data: $state.candidate
  )
  |> if $state.ok == true then transition $state.status -> valid else transition $state.status -> invalid { missing: $state.missing }
}
`,
		output: `{
  "missing": ["team"],
  "ok": false,
  "state.status": "invalid"
}`,
		steps: 5,
		tags: ['validate_schema', 'if', 'transition']
	},
	{
		id: 'yaml',
		label: 'YAML → JSON',
		title: 'Config in, structure out',
		blurb: 'The wasm-safe stdlib ships text transforms: yaml, json, csv, html → markdown.',
		source: `import core from "grapheme/core"
import yaml from "grapheme/yaml"

query Yaml {
  set { text: "service: checkout\\nreplicas: 3\\nregions: [us-east, eu-west]" }
  |> yaml.to_json(text: $state.text)
}
`,
		output: `{
  "regions": ["us-east", "eu-west"],
  "replicas": 3,
  "service": "checkout"
}`,
		steps: 2,
		tags: ['yaml']
	},
	{
		id: 'policy',
		label: 'Policy (fails closed)',
		title: 'Side effects are a capability, not a default',
		blurb:
			'In the browser runtime, `http` is outside the Wasm stdlib profile. The runtime refuses — the same way a production host refuses without an allow-list.',
		source: `import http from "grapheme/http"

query NeedsHttp {
  http.get(url: "https://example.com")
}
`,
		output: `capability 'http.get' is outside the Wasm stdlib profile;
host must provide grapheme.runtime.host.v1::call.capability`,
		steps: 1,
		tags: ['policy', 'capability', 'http']
	}
];

export const HERO_SNIPPET = SNIPPETS[0]!;

export function snippetById(id: string): Snippet | undefined {
	return SNIPPETS.find((s) => s.id === id);
}
