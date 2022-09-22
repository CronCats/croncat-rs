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
    # Inside cw-croncat
    docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    --platform linux/amd64 \
    cosmwasm/rust-optimizer:0.12.8

    # Copy wasms to the docker container
    docker cp artifacts/cw_croncat.wasm juno-node-1:/cw_croncat.wasm
    docker cp artifacts/cw_rules.wasm juno-node-1:/cw_rules.wasm

    # Back to original terminal

    # Copy cw20 to the docker container
    docker cp docs/cw20_base.wasm juno-node-1:/cw20_base.wasm
    
    # Store all contracts
    CODE_ID=$($BINARY tx wasm store "/cw_croncat.wasm" --from validator $TXFLAG --output json | jq -r '.logs[0].events[-1].attributes[0].value')
    RULES_ID=$($BINARY tx wasm store "/cw_rules.wasm" --from validator $TXFLAG --output json | jq -r '.logs[0].events[-1].attributes[0].value')
    CW20_ID=$($BINARY tx wasm store "/cw20_base.wasm" --from validator $TXFLAG --output json | jq -r '.logs[0].events[-1].attributes[0].value')
    echo -e "CW_CRONCAT: $CODE_ID\nCW_RULES: $RULES_ID\nCW20: $CW20_ID"

    # Instantiate cw_rules
    $BINARY tx wasm instantiate $RULES_ID '{}' --from validator --label "cw_rules" $TXFLAG -y --no-admin
    CW_RULES_ADDR=$($BINARY q wasm list-contract-by-code $RULES_ID --output json | jq -r '.contracts[-1]')
    echo $CW_RULES_ADDR
    # Instantiate cw_croncat
    INIT='{"denom":"ujunox", "cw_rules_addr": "'$CW_RULES_ADDR'"}'
    $BINARY tx wasm instantiate $CODE_ID "$INIT" --from validator --label "croncat" $TXFLAG -y --no-admin
    # Instantiate cw20
    INIT_CW20='{"name": "memecoin", "symbol": "meme", "decimals": 4, "initial_balances": [{"address": "'$($BINARY keys show validator -a)'", "amount": "100000"}]}'
    $BINARY tx wasm instantiate $CW20_ID "$INIT_CW20" --from validator --label "memecoin" $TXFLAG -y --no-admin
    # Get contract address
    CONTRACT_ADDRESS=$($BINARY q wasm list-contract-by-code $CODE_ID --output json | jq -r '.contracts[-1]')
    echo $CONTRACT_ADDRESS
    # Get cw20 addr
    CW20_ADDR=$($BINARY q wasm list-contract-by-code $CW20_ID --output json | jq -r '.contracts[-1]')
    echo $CW20_ADDR
    ```
5. Edit `croncat-rs` for new croncat addr
6. Create and store new agent addr
   ```bash
   # Inside croncat-rs
   cargo run -- --chain-id local generate-mnemonic
   # Store your agent addr
   AGENT_ADDR=juno18luucfmwyqta72u4qj4wt6dc4jwlgwcgzvw0jp
   ```
7. Refill agent's balance, for `register-agent` and `proxy-call`s
   ```bash
   $BINARY tx bank send validator $AGENT_ADDR 100000000ujunox $TXFLAG
   ```
8. Register agent
   ```bash
   cargo run -- --chain-id local register-agent
   ```
9. Generate some random wallet and save address
   ```bash
   $BINARY keys add test
   BOB=$($BINARY keys show test -a)
   ```
10. Create bob on-chain
    ```bash
    $BINARY tx bank send validator $BOB 1ujunox $TXFLAG
    ```
11. Transfer cw20 to create task with cw20 transfer
    ```bash
    CW20_SEND='{"send": {"contract": "'$CONTRACT_ADDRESS'", "amount": "5", "msg": ""}}'
    $BINARY tx wasm execute $CW20_ADDR "$CW20_SEND" --from validator $TXFLAG -y
    ```
12. Start go
    ```bash
    cargo run -- --chain-id local go -r
    ``` 
13. Add new task:
    ```bash
    BASE64_TRANSFER=$(echo '{"transfer":{"recipient":"'$AGENT_ADDR'","amount":"5"}}' | base64 -w 0)
    RULES='{
        "create_task": {
            "task": {
                "interval": "Once",
                "boundary": null,
                "stop_on_fail": false,
                "actions": [
                    {
                        "msg": {
                            "wasm": {
                                "execute": {
                                    "contract_addr": "'$CW20_ADDR'",
                                    "msg": "'$BASE64_TRANSFER'",
                                    "funds": []
                                }
                            }
                        },
                        "gas_limit": null
                    }
                ],
                "rules": [
                    {
                        "has_balance_gte": {
                            "address": "'$BOB'",
                            "required_balance": {
                                "cw20": {
                                    "address": "'$CW20_ADDR'",
                                    "amount": "5"
                                }
                            }
                        }
                    }
                ],
                "cw20_coins": [
                    {
                        "address": "'$CW20_ADDR'",
                        "amount": "5"
                    }
                ]
            }
        }
    }'
    $BINARY tx wasm execute $CONTRACT_ADDRESS "$RULES" --amount 1700004ujunox --from validator $TXFLAG -y
    ```
14. Transfer to bob 5memecoins to activate rules
    ```bash
    CW20_TRANSFER='{"transfer": {"recipient": "'$BOB'", "amount": "5"}}'
    $BINARY tx wasm execute $CW20_ADDR "$CW20_TRANSFER" --from validator $TXFLAG -y
    ```
15. Ensure Agent's cw20 balance updated
    ```bash
    $BINARY query wasm contract-state smart $CW20_ADDR '{"balance": {"address": "'$AGENT_ADDR'"}}'
    # Expected to be 5
    ```