<div align="center">

  <h1><code>Clarus Node</code></h1>

  <strong>Clarus Blockchain using <a href="https://github.com/paritytech/substrate">Substrate</a>.</strong>

  <h3>
    <a href="https://substrate.io/">Docs</a>
    <span> | </span>
    <a href="https://matrix.to/#/!HzySYSaIhtyWrwiwEV:matrix.org?via=matrix.parity.io&via=matrix.org&via=web3.foundation">Chat</a>
  </h3>

</div>



## Getting Started

Follow the steps below to get started.

### Rust Setup

First, complete the [Dev Docs Installation](https://docs.substrate.io/v3/getting-started/installation/).

### Build and Run

Use the following command to build the node and run it after build successfully:

```sh
cargo build --release
./target/release/clarus-node --dev
```

## Run public testnet

* Modify the genesis config in chain_spec.rs
* Build spec, `./target/release/clarus-node build-spec --chain staging > clarus-staging.json`
* Change original spec to encoded raw spec, `./target/release/clarus-node build-spec --chain=clarus-staging.json --raw > clarus-staging-raw.json`
* Start your bootnodes, node key can be generate with command `./target/release/clarus-node key generate-node-key`.
  ```shell
  ./target/release/clarus-node \
       --node-key <your-node-key> \
       --base-path /tmp/bootnode1 \
       --chain clarus-staging-raw.json \
       --name bootnode1
  ```
* Start your initial validators,
  ```shell
  ./target/release/clarus-node \
      --base-path  /tmp/validator1 \
      --chain   clarus-staging-raw.json \
      --bootnodes  /ip4/<your-bootnode-ip>/tcp/30333/p2p/<your-bootnode-peerid> \
	    --port 30336 \
	    --ws-port 9947 \
	    --rpc-port 9936 \
      --name  validator1 \
      --validator
  ```
* [Insert session keys](https://substrate.dev/docs/en/tutorials/start-a-private-network/customchain#add-keys-to-keystore)
* Attract enough validators from community in waiting
* Call force_new_era in staking pallet with sudo, rotate to PoS validators
* Enable governance, and remove sudo
* Enable transfer and other functions
