<div align="center">
  <h1>Vesu Liquidator</h1>
  <img src="docs/images/logo.webp" height="400" width="400">
  <br />
  <a href="https://github.com/astraly-labs/Vesu-liquidator/issues/new?assignees=&labels=bug&template=01_BUG_REPORT.md&title=bug%3A+">Report a Bug</a>
  -
  <a href="https://github.com/astraly-labs/Vesu-liquidator/issues/new?assignees=&labels=enhancement&template=02_FEATURE_REQUEST.md&title=feat%3A+">Request a Feature</a>
</div>

## About

`vesu-liquidator` is an automated bot that monitors positions on the Vesu Protocol and automatically liquidates them if it's worth it.

## Getting Started

### Prerequisites

#### Protobuf

In order to run the liquidator, you need the protoc Protocol Buffers compiler, along with Protocol Buffers resource files.

##### Ubuntu

```sh
sudo apt update && sudo apt upgrade -y
sudo apt install -y protobuf-compiler libprotobuf-dev
```

##### macOS

Assuming Homebrew is already installed.

```sh
brew install protobuf
```

#### Environment Variables

Create an `.env` file following the example file and fill the keys.

## Usage

### Build

```sh
cargo build --release
```

The executable can be found at `./target/release/vesu-liquidator`.

### Run

You can run `vesu-liquidator --help` - which will show how to use the bot:

```bash
Usage: vesu-liquidator [OPTIONS] --account-address <LIQUIDATOR ACCOUNT ADDRESS> --network <NETWORK NAME> --rpc-url <RPC URL> --starting-block <BLOCK NUMBER> --pragma-api-base-url <PRAGMA API BASE URL>

Options:
      --account-address <LIQUIDATOR ACCOUNT ADDRESS>
          Account address of the liquidator account

      --private-key <LIQUIDATOR PRIVATE KEY>
          Private key of the liquidator account

      --keystore-path <LIQUIDATOR KEYSTORE>
          Keystore path for the liquidator account

      --keystore-password <LIQUIDATOR KEYSTORE PASSWORD>
          Keystore password for the liquidator account

  -n, --network <NETWORK NAME>
          The network chain configuration [possible values: mainnet, sepolia]

      --rpc-url <RPC URL>
          The rpc endpoint url

      --config-path <VESU CONFIG PATH>
          Configuration file path [default: config.yaml]

  -s, --starting-block <BLOCK NUMBER>
          The block you want to start syncing from

      --pragma-api-base-url <PRAGMA API BASE URL>
          Pragma API Key for indexing

      --apibara-api-key <APIBARA API KEY>
          Apibara API Key for indexing

      --pragma-api-key <PRAGMA API KEY>
          Pragma API Key for indexing

  -h, --help
          Print help
```

#### Example - running the bot on Mainnet

```bash
./target/release/vesu-liquidator --network mainnet --rpc-url https://starknet-mainnet.public.blastapi.io --starting-block 668886 --pragma-api-base-url https://api.dev.pragma.build --account-address <YOUR_ACCOUNT> --private-key <YOUR_PRIVATE_KEY>
```

Should run the bot:

```bash


██╗   ██╗███████╗███████╗██╗   ██╗    ██╗     ██╗ ██████╗ ██╗   ██╗██╗██████╗  █████╗ ████████╗ ██████╗ ██████╗
██║   ██║██╔════╝██╔════╝██║   ██║    ██║     ██║██╔═══██╗██║   ██║██║██╔══██╗██╔══██╗╚══██╔══╝██╔═══██╗██╔══██╗
██║   ██║█████╗  ███████╗██║   ██║    ██║     ██║██║   ██║██║   ██║██║██║  ██║███████║   ██║   ██║   ██║██████╔╝
╚██╗ ██╔╝██╔══╝  ╚════██║██║   ██║    ██║     ██║██║▄▄ ██║██║   ██║██║██║  ██║██╔══██║   ██║   ██║   ██║██╔══██╗
 ╚████╔╝ ███████╗███████║╚██████╔╝    ███████╗██║╚██████╔╝╚██████╔╝██║██████╔╝██║  ██║   ██║   ╚██████╔╝██║  ██║
  ╚═══╝  ╚══════╝╚══════╝ ╚═════╝     ╚══════╝╚═╝ ╚══▀▀═╝  ╚═════╝ ╚═╝╚═════╝ ╚═╝  ╚═╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝

  🤖 Liquidator 👉 0x42f09c629f993bd4ce1f6524c24aed223c7c4b967d732a9a4674cf07088cc6c
  🎯 On Mainnet
  🥡 Starting from block 668886


2024-08-22T11:09:09.432898Z  INFO ThreadId(01) src/services/mod.rs:35: 🧩 Starting the indexer service...
2024-08-22T11:09:09.433057Z  INFO ThreadId(01) src/services/mod.rs:44: ⏳ Waiting a few moment for the indexer to fetch positions...

2024-08-22T11:09:09.433089Z  INFO ThreadId(01) src/services/mod.rs:47: 🧩 Starting the oracle service...

2024-08-22T11:09:09.447471Z  INFO ThreadId(01) src/services/mod.rs:54: 🧩 Starting the monitoring service...

# rest of the execution...
```
