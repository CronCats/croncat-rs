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

## Help

```
$ cargo run help
...
croncatd 0.3.0
The croncat agent daemon.

USAGE:
    croncatd [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -d, --debug        Debug mode
    -h, --help         Prints help information
        --no-frills    Whether to print nice little things like the banner and a goodbye
    -V, --version      Prints version information

OPTIONS:
        --agent <agent>          ID of the agent config to use [env: CRONCAT_AGENT=]  [default: agent]
        --chain-id <chain-id>    Chain ID of the chain to connect to [env: CRONCAT_CHAIN_ID=uni-6]

SUBCOMMANDS:
    all-tasks            Get contract's state Show all task(s) information
    generate-mnemonic    Generates a new keypair and agent account (good first step)
    get-agent-keys       [SENSITIVE!] Shows all details about agents on this machine
    get-tasks            Get the agent's tasks they're assigned to fulfill
    go                   Starts the Croncat agent, allowing it to fulfill tasks
    help                 Prints this message or the help of the given subcommand(s)
    list-accounts        Get the agent's supported bech32 accounts
    register             Registers an agent, placing them in the pending queue unless it's the first agent
    send                 Send funds from the agent account to another account
    setup-service        Setup an agent as a system service (systemd)
    status               Get the agent's status (pending/active)
    unregister           Unregisters the agent from being in the queue with other agents
    update               Update the agent's configuration
    withdraw             Withdraw the agent's funds to the payable account ID

Example:
$ cargo run -- --debug status

```

## Generate Docs

```bash
cargo doc --no-deps
```

## Setup

-   Set a contract address for each chain in config.yaml
-   Add agent into local keystore

```bash
## Generate for a specific network
## "mainnet" gives a namespace to a key/pair.
## It is advised to create separate keys for mainnet/testnet
cargo run generate-mnemonic mainnet

## Another way to load key/pair
cargo run generate-mnemonic mainnet --mnemonic "olive soup parade family educate congress hurt dwarf mom this position hungry unaware aunt swamp sunny analyst wrestle fashion main knife start coffee air"
```

### Register an agent

```bash
export CRONCAT_CHAIN_ID=uni-6
export CRONCAT_AGENT=mainnet
cargo run register
```

### Go for executing tasks

```bash
cargo run go
```

### Claim Rewards

After a while, time to claim some rewards if you've been actively processing tasks!

```bash
# Check if you have any rewards, see "Earned Rewards"
# NOTE: Also shows how much balance your agent has for processing txns
cargo run status

# Claim rewards
cargo run withdraw
```

### Send Funds

Maybe you wanna send claimed rewards elsewhere, simple command for this! Just make sure you leave funds for the agent to sign txn fees...

```bash
cargo run send juno1x4uaf...8q8jdraaqj 10 ujunox
```

### Configuring Custom RPCs

```
    uni-6:
        factory: juno1x4uaf50flf6af8jpean8ruu8q8jdraaqj7e3gg3wemqm5cdw040qk982ec
        gas_prices: 0.04
        gas_adjustment: 1.3
        rpc_timeout: 9.0
        include_evented_tasks: false
        custom_sources:
            "Cats R US 🙀":
                rpc: http://192.168.1.13
```

## Code of Conduct

-   Please see [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md)

## Contributing

-   Please see [CONTRIBUTING.md](./CONTRIBUTING.md)

### Chain Registry:

For clearing the latest local cache of chain registry, `rm -rf .cosmos-chain-registry`, then build.

### This project is made possible by these awesome contributors!

<a href="https://github.com/CronCats/croncat-rs/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=CronCats/croncat-rs" />
</a>
