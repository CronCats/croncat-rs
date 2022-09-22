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
Command line subcommands

USAGE:
    croncatd [FLAGS] --chain-id <chain-id> <SUBCOMMAND>

FLAGS:
    -d, --debug        Debug mode
    -h, --help         Prints help information
        --no-frills    Whether to print nice little things like the banner and a goodbye
    -V, --version      Prints version information

OPTIONS:
        --chain-id <chain-id>

SUBCOMMANDS:
    deposit-ujunox       (in progress) Send native tokens to an address
    generate-mnemonic    Generates a new keypair and agent account (good first step)
    get-agent            Sensitive. Shows all details about agents on this machine
    get-agent-status     Get the agent's status (pending/active)
    get-agent-tasks      Get the agent's tasks they're assigned to fulfill
    go                   Starts the Croncat agent, allowing it to fulfill tasks
    help                 Prints this message or the help of the given subcommand(s)
    info                 Gets the configuration from the Croncat manager contract
    register-agent       Registers an agent, placing them in the pending queue unless it's the first agent
    setup-service        Setup an agent as a system service (systemd)
    status               (in progress) Get the agent's status
    tasks                Show all task(s) information
    unregister-agent     Unregisters the agent from being in the queue with other agents
    update-agent         Update the agent's configuration
    withdraw             Withdraw the agent's funds to the payable account ID
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
  <tr>
    <td align="center"><a href="http://seedyrom.io"><img src="https://avatars.githubusercontent.com/u/11783357?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Zack Kollar</b></sub></a><br /><a href="https://github.com/CronCats/croncat-rs/commits?author=SeedyROM" title="Code">💻</a> <a href="https://github.com/CronCats/croncat-rs/issues?q=author%3ASeedyROM" title="Bug reports">🐛</a> <a href="#example-SeedyROM" title="Examples">💡</a> <a href="#ideas-SeedyROM" title="Ideas, Planning, & Feedback">🤔</a> <a href="#question-SeedyROM" title="Answering Questions">💬</a> <a href="#talk-SeedyROM" title="Talks">📢</a> <a href="https://github.com/CronCats/croncat-rs/pulls?q=is%3Apr+reviewed-by%3ASeedyROM" title="Reviewed Pull Requests">👀</a> <a href="#content-SeedyROM" title="Content">🖋</a></td>
    <td align="center"><a href="http://gitlab.com/TrevorJTClarke"><img src="https://avatars.githubusercontent.com/u/2633184?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Trevor Clarke</b></sub></a><br /><a href="https://github.com/CronCats/croncat-rs/commits?author=TrevorJTClarke" title="Code">💻</a></td>
  </tr>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!
