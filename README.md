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

## Contributors ✨

Thanks goes to these wonderful people ([emoji key](https://allcontributors.org/docs/en/emoji-key)):

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center"><a href="http://seedyrom.io"><img src="https://avatars.githubusercontent.com/u/11783357?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Zack Kollar</b></sub></a><br /><a href="https://github.com/CronCats/croncat-rs/commits?author=SeedyROM" title="Code">💻</a> <a href="https://github.com/CronCats/croncat-rs/issues?q=author%3ASeedyROM" title="Bug reports">🐛</a> <a href="#example-SeedyROM" title="Examples">💡</a> <a href="#ideas-SeedyROM" title="Ideas, Planning, & Feedback">🤔</a> <a href="#question-SeedyROM" title="Answering Questions">💬</a> <a href="#talk-SeedyROM" title="Talks">📢</a> <a href="https://github.com/CronCats/croncat-rs/pulls?q=is%3Apr+reviewed-by%3ASeedyROM" title="Reviewed Pull Requests">👀</a> <a href="#content-SeedyROM" title="Content">🖋</a></td>
      <td align="center"><a href="http://gitlab.com/TrevorJTClarke"><img src="https://avatars.githubusercontent.com/u/2633184?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Trevor Clarke</b></sub></a><br /><a href="https://github.com/CronCats/croncat-rs/commits?author=TrevorJTClarke" title="Code">💻</a></td>
      <td align="center"><a href="http://deveus.me/"><img src="https://avatars.githubusercontent.com/u/42238266?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Raf Deveus</b></sub></a><br /><a href="https://github.com/CronCats/croncat-rs/commits?author=deveusss" title="Code">💻</a> <a href="https://github.com/CronCats/croncat-rs/issues?q=author%3Adeveusss" title="Bug reports">🐛</a> <a href="#ideas-deveusss" title="Ideas, Planning, & Feedback">🤔</a> <a href="#example-deveusss" title="Examples">💡</a> <a href="#question-deveusss" title="Answering Questions">💬</a> <a href="#talk-deveusss" title="Talks">📢</a> <a href="https://github.com/CronCats/croncat-rs/pulls?q=is%3Apr+reviewed-by%3Adeveusss" title="Reviewed Pull Requests">👀</a></td>
    </tr>
  </tbody>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!
