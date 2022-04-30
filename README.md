&nbsp;

<div align="center">
<img width="600" src="./croncat.png" />
</div>

---

# croncat-rs

`croncat-rs` is the brand new version of the croncat agent, written in Rust.

## Modules:

-   `croncatd` This is the executable agent daemon.
-   `croncat` This is all the pieces to build an agent daemon, this will probably become it's own repo so keep it DRY and clean.

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
        --no-frills    Whether to print nice little things like the banner and a goodbye
    -V, --version      Prints version information
```

## Generate Docs:

-   `cargo doc --no-deps`

## Contributors ✨

Thanks goes to these wonderful people ([emoji key](https://allcontributors.org/docs/en/emoji-key)):

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tr>
    <td align="center"><a href="http://seedyrom.io"><img src="https://avatars.githubusercontent.com/u/11783357?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Zack Kollar</b></sub></a><br /><a href="https://github.com/SeedyROM/croncat-rs/commits?author=SeedyROM" title="Code">💻</a> <a href="https://github.com/SeedyROM/croncat-rs/issues?q=author%3ASeedyROM" title="Bug reports">🐛</a> <a href="#example-SeedyROM" title="Examples">💡</a> <a href="#ideas-SeedyROM" title="Ideas, Planning, & Feedback">🤔</a> <a href="#question-SeedyROM" title="Answering Questions">💬</a> <a href="#talk-SeedyROM" title="Talks">📢</a> <a href="https://github.com/SeedyROM/croncat-rs/pulls?q=is%3Apr+reviewed-by%3ASeedyROM" title="Reviewed Pull Requests">👀</a></td>
  </tr>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!
