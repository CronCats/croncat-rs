1. Clone juno with open grpc https://github.com/Buckram123/juno/tree/grpc
   - tree grpc
2. Start juno: `STAKE_TOKEN=ujunox UNSAFE_CORS=true docker-compose up`
3. Configure
    ```bash 
    TXFLAG="--gas-prices 0.1ujunox --gas auto --gas-adjustment 1.3 -y -b block --chain-id testing --node http://localhost:26657/"
    # if you are on an M1 mac you will have to run those commands _inside_ the container
    # notice that juno_node_1 is the default
    # on e.g. a linux box with juno installed
    # the correct ports will be open to talk to juno running in docker
    # even so, proably easier to use $BINARY as below
    BINARY="docker exec -i juno-node-1 junod"
    ```
4. Compile and deploy your croncat
    ```bash
    #Inside cw-croncat
    docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    --platform linux/amd64 \
    cosmwasm/rust-optimizer:0.12.8

    # Copy wasms to the docker container
    docker cp artifacts/cw_croncat.wasm juno-node-1:/cw_croncat.wasm
    docker cp artifacts/cw_rules.wasm juno-node-1:/cw_rules.wasm

    # Back to original terminal
    # Store both contracts
    CODE_ID=$($BINARY tx wasm store "/cw_croncat.wasm" --from validator $TXFLAG --output json | jq -r '.logs[0].events[-1].attributes[0].value')
    RULES_ID=$($BINARY tx wasm store "/cw_rules.wasm" --from validator $TXFLAG --output json | jq -r '.logs[0].events[-1].attributes[0].value')
    echo -e "CW_CRONCAT: $CODE_ID\nCW_RULES: $RULES_ID"

    # Instantiate cw_rules
    $BINARY tx wasm instantiate $RULES_ID '{}' --from validator --label "cw_rules" $TXFLAG -y --no-admin
    CW_RULES_ADDR=$($BINARY q wasm list-contract-by-code $RULES_ID --output json | jq -r '.contracts[-1]')
    echo $CW_RULES_ADDR
    # Instantiate cw_croncat
    INIT='{"denom":"ujunox", "cw_rules_addr": "'$CW_RULES_ADDR'"}'
    $BINARY tx wasm instantiate $CODE_ID "$INIT" --from validator --label "croncat" $TXFLAG -y --no-admin

    # Get contract address
    CONTRACT_ADDRESS=$($BINARY q wasm list-contract-by-code $CODE_ID --output json | jq -r '.contracts[-1]')
    echo $CONTRACT_ADDRESS
    ```
5. Edit `croncat-rs` for new croncat addr
6. Create and store new agent addr
   ```bash
   # Inside croncat-rs
   cargo run -- --network local generate-mnemonic
   # Store your agent addr
   AGENT_ADDR=juno18luucfmwyqta72u4qj4wt6dc4jwlgwcgzvw0jp
   ```
7. Refill agent's balance, for `register-agent` and `proxy-call`s
   ```bash
   $BINARY tx bank send validator $AGENT_ADDR 100000000ujunox $TXFLAG
   ```
8. Register first agent
   ```bash
   cargo run -- --network local register-agent
   ```
9. Now lets add mike-agent
    ```bash
    cargo run -- --network local generate-mnemonic --new-name mike
    # Store Mike addr
    MIKE_ADDR=juno1n6ns7urmgslq2mjl9qx49rn0a9504m23jdrn3x
    ```
10. Refill Mike's balance and start daemon as Mike 
    ```bash
    $BINARY tx bank send validator $MIKE_ADDR 100000000ujunox $TXFLAG
    # And start daemon
    cargo run -- --network local daemon --sender-name mike
    ```
11. Unregister first agent
    ```bash
    cargo run -- --network local unregister-agent
    ```
12. Add new task:
    ```bash
    RECURRING='{
    "create_task": {
      "task": {
        "interval": {
          "Block": 15
        },
        "boundary": null,
        "cw20_coins": [],
        "stop_on_fail": false,
        "actions": [
          {
            "msg": {
              "bank": {
                "send": {
                  "amount": [
                    {
                      "amount": "1",
                      "denom": "ujunox"
                    }
                  ],
                  "to_address": "juno1yhqft6d2msmzpugdjtawsgdlwvgq3samrm5wrw"
                }
              }
            }
          },
          {
            "msg": {
              "bank": {
                "send": {
                  "amount": [
                    {
                      "amount": "1",
                      "denom": "ujunox"
                    }
                  ],
                  "to_address": "juno15w7hw4klzl9j2hk4vq7r3vuhz53h3mlzug9q6s"
                }
              }
            }
          }
        ],
        "rules": []
      }
    }
    }'
    $BINARY tx wasm execute $CONTRACT_ADDRESS "$RECURRING" --amount 1700004ujunox --from validator $TXFLAG -y
    ```