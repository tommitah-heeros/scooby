# Word of caution
This is the authors "first" project in Rust, it sucks.

## Running turso locally
```bash
turso dev --db-file <path>
```
This outputs a port, which you will feed into:

```bash
turso db shell <port>
```

Turso shell opens, query away!
