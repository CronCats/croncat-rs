&nbsp;

<div align="center">
<img width="600" src="./croncat.png" />
</div>

&nbsp;

---

# croncat-rs

`croncat-rs` is the brand new version of the croncat agent, written in Rust.

## Modules

-   `croncatd` This is the executable agent daemon.
-   `croncat` This is all the pieces to build an agent daemon, this will probably become it's own repo so keep it DRY and clean.

## Run

-   `cargo run`

## Help

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
        --no-frills    Whether to print nice little things like the banner and a goodbye
    -V, --version      Prints version information
```

## Generate Docs

-   `cargo doc --no-deps`

## Contributing

-   Please see [CONTRIBUTING.md](./CONTRIBUTING.md)

## Code of Conduct

-   Please see [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)

## Maintainers

<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tr>
    <td align="center">
      <a href="http://seedyrom.io"
        ><img
          src="https://avatars.githubusercontent.com/u/11783357?v=4&s=100"
          width="100px;"
          alt=""
        /><br /><sub><b>Zack Kollar</b></sub></a
      >
    </td>
    <td align="center">
      <a href="http://gitlab.com/TrevorJTClarke"
        ><img
          src="https://avatars.githubusercontent.com/u/2633184?v=4&s=100"
          width="100px;"
          alt=""
        /><br /><sub><b>Trevor Clarke</b></sub></a
      >
    </td>
    <td align="center">
      <a href="http://gitlab.com/mikedotexe"
        ><img
          src="https://avatars.githubusercontent.com/u/1042667?v=4&s=100"
          width="100px;"
          alt=""
        /><br /><sub><b>Mike Pervis</b></sub></a
      >
    </td>
    <td align="center">
      <a href="http://github.com/deveusss"
        ><img
          src="https://avatars.githubusercontent.com/u/42238266?v=4&s=100"
          width="100px;"
          alt=""
        /><br /><sub><b>Raf Deveus</b></sub></a
      >
    </td>
    <td align="center">
      <a href="http://github.com/Buckram123"
        ><img
          src="https://avatars.githubusercontent.com/u/91957742?v=4&s=100"
          width="100px;"
          alt=""
        /><br /><sub><b>Mykhailo Donchenko</b></sub></a
      >
    </td>
  </tr>
</table>
<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->
