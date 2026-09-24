# trdc Dependencies Summary

## Package Name
`trdc`

## Dependencies

| Crate | Version / Features |
|-------|-------------------|
| `clap` | 4.0 with "derive" feature |
| `anyhow` | 1.0 |
| `serde` | 1.x with "derive" feature |
| `serde_json` | 1.x |
| `dirs` | 6.0 |
| `toml` | 1.x |
| `chrono` | 0.4 |
| `reqwest` | 0.13.2 with json and stream features |
| `tokio` | 1.x with rt-multi-thread, macros, io-util features |
| `futures-util` | 0.3 |
| `regex` | 1.x |

## Profile Configuration (release)
- `opt-level = 3`
- `lto = true`
- `codegen-units = 1`
- `panic = "abort"`
- `strip = true`