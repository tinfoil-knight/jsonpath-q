# JSONPath Query Interpreter (WIP)

A work-in-progress interpreter for JSONPath Query Expressions as described in [RFC 9535](https://www.rfc-editor.org/info/rfc9535).

## Usage

### Running

Create a build using `cargo build --release` or use `cargo run`.

```
jsonpath-q -q <query> [-f <filepath>]
```

```
Options:
  -q, --query       query eg: "$['foo'].[1]"
  -f, --filepath    filepath
  --help, help      display usage information
```

> You can also stdin to provide input. Eg: `cat data.json | jsonpath-q -q <query>`.

### Testing

All tests are currently in `src/lib.rs`. To run them, use `cargo test`.

## Author

- Kunal Kundu - [@tinfoil-knight](https://github.com/tinfoil-knight)

## License

Distributed under the MIT License. See [LICENSE](./LICENSE) for more information.

## References
- S. Gössner, G. Normington, C. Bormann. 2024. ["JSONPath: Query Expressions for JSON", RFC 9535](https://www.rfc-editor.org/info/rfc9535)
