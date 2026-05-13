# Transform Cookbook

Short, practical examples for common transform pipelines.

## Examples

- `http-html-markdown.gr`
  - Fetch HTML over HTTP and convert body text to markdown.
- `yaml-json-parse-field.gr`
  - Parse YAML, then explicitly chain a string field into `json.parse` with `$current.payload`.
- `csv-to-json-envelope.gr`
  - Parse CSV rows and wrap them into an object envelope for downstream steps.
- `core-string-ops.gr`
  - Compose trim/lower/replace/split/join/upper helpers in one string pipeline.
- `core-list-ops.gr`
  - Group list records by a chosen field with `core.group_by`.
- `core-reduce-modes.gr`
  - Compute summary values with `core.reduce` modes such as `avg`, `min`, and `max`.
- `core-path-ops.gr`
  - Write and read nested object values with `core.set_path` and `core.get_path`.

Run any recipe with:

```bash
cargo run -- run examples/legacy/transform-cookbook/<file>.gr --native-modules --json
```
