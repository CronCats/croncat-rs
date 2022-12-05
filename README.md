&nbsp;

<div align="center">
<img width="300px" src="./croncat.png" />
</div>

&nbsp;

---

# croncat-rs

`croncat-rs` is the brand new version of the croncat agent, written in Rust.

## Modules

-   `croncatd` The executable agent daemon.
-   `croncat` All the pieces to build an agent daemon.

## Development Tools

-   `cargo install rusty-hook cargo-make`
-   `rusty-hook init`

## Run

-   `cargo run`

## Help

```
$ cargo run -- --help
...
croncatd 0.1.0
The croncat agent daemon.

USAGE:
    croncatd [FLAGS] <SUBCOMMAND>

FLAGS:
    -d, --debug        Debug mode
    -h, --help         Prints help information
        --no-frills    Whether to print nice little things like the banner and a goodbye
    -V, --version      Prints version information

SUBCOMMANDS:
    generate-mnemonic    Generates a new keypair and agent account (good first step)
    get-agent            Sensitive. Shows all details about agents on this machine
    get-agent-accounts   Shows all the natively supported bech32 accounts for a key pair
    get-agent-status     Get the agent's status (pending/active)
    get-agent-tasks      Get the agent's tasks they're assigned to fulfill
    go                   Starts the Croncat agent, allowing it to fulfill tasks
    help                 Prints this message or the help of the given subcommand(s)
    info                 Gets the configuration from the Croncat manager contract
    register-agent       Registers an agent, placing them in the pending queue unless it's the first agent
    setup-service        Setup an agent as a system service (systemd)
    tasks                Show all task(s) information
    unregister-agent     Unregisters the agent from being in the queue with other agents
    update-agent         Update the agent's configuration
    withdraw             Withdraw the agent's funds to the payable account ID
```

## Generate Docs

```bash
cargo doc --no-deps
```

- Set contract address in config.local.yaml
- Add agent into local keystore

```bash 
## Generate for a specific network
## "new-name" gives a namespace to a key/pair.
## It is advised to create separate keys for mainnet/testnet
cargo run generate-mnemonic --new-name mainnet

## Another way to load key/pair
cargo run generate-mnemonic --mnemonic "olive soup parade family educate congress hurt dwarf mom this position hungry unaware aunt swamp sunny analyst wrestle fashion main knife start coffee air"
```

### Register an agent
```bash
cargo run register-agent --chain-id "local"
```

### Go for executing tasks
```bash
cargo run go --chain-id "local"
```


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
