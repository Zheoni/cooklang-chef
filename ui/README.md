# Web UI

To build the styles you need the [`tailwindcss` CLI](https://tailwindcss.com/blog/standalone-cli). (Version `v3.4.0` was used in development)

```sh
# For development
tailwindcss -i input.css -o assets/styles.css --watch
cargo watch -w ui/templates -w src -w ui/assets -- cargo run -- serve

# Production build
tailwindcss -i input.css -o assets/styles.css --minify
```

To update vendored dependencies run

```sh
node frontendMappings.js
```
