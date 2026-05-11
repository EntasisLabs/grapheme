# Transform Cookbook

Short, practical examples for common transform pipelines.

## Examples

- `http-html-markdown.gr`
  - Fetch HTML over HTTP and convert body text to markdown.
- `yaml-json-parse-field.gr`
  - Parse YAML, then explicitly chain a string field into `json.parse` with `$current.payload`.
- `csv-to-json-envelope.gr`
  - Parse CSV rows and wrap them into an object envelope for downstream steps.

Run any recipe with:

```bash
cargo run -- run examples/transform-cookbook/<file>.gr --native-modules --json
```
