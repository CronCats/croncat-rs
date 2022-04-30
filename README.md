<div align="center">
<img width="600" src="./croncat.png" />
</div>

---

# croncat-rs

`croncat-rs` is the brand new version of the croncat agent, written in Rust.

## Modules:

-   `croncatd` This is the executable agent daemon.
-   `croncat` This is all the pieces to build an agent daemon.

## Run:

-   `cargo run`

## Help:

```
$ cargo run -- --help
...
croncatd 0.1.0
The croncat agent daemon.

USAGE:
    croncatd [FLAGS]

FLAGS:
    -d, --debug        Debug mode
    -h, --help         Prints help information
        --no-frills    Wether to print nice little things like the banner and a goodbye
    -V, --version      Prints version information
```

## Generate Docs:

-   `cargo doc --no-deps`
