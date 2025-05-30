<div align="center">
  <h1>Vesu Liquidator</h1>
  <img src="docs/images/logo.jpeg" height="400" width="400">
  <br />
  <a href="https://github.com/astraly-labs/Vesu-liquidator/issues/new?assignees=&labels=bug&template=01_BUG_REPORT.md&title=bug%3A+">Report a Bug</a>
  -
  <a href="https://github.com/astraly-labs/Vesu-liquidator/issues/new?assignees=&labels=enhancement&template=02_FEATURE_REQUEST.md&title=feat%3A+">Request a Feature</a>
</div>

## About

Vesu Liquidator 🤖 is an automated bot that monitors positions on the Vesu Protocol and liquidates them.

## Getting Started

### Docker through published package

You can run the Vesu Liquidator using our pre-built Docker image. Here's how to use it:

1. Pull the latest image:

```sh
docker pull ghcr.io/astraly-labs/vesu-liquidator:latest
```

1. Run the container:

```sh
docker run --rm -it \
  -v /path/to/your/.env:/app/.env \
  ghcr.io/astraly-labs/vesu-liquidator:latest \
  --account-address <LIQUIDATOR_ACCOUNT_ADDRESS> \
  --network <NETWORK_NAME> \
  --rpc-url <RPC_URL> \
  --starting-block <BLOCK_NUMBER> \
  --storage-path ./vesu.json
```

For more options, run:

```bash
docker run --rm ghcr.io/astraly-labs/vesu-liquidator:latest --help
```

### Docker locally

If you want to build the Docker image locally:

1. Build the Docker image:

```sh
docker build -t vesu-liquidator .
```

2. Run the locally built image:

```sh
docker run --rm vesu-liquidator --help
# OR
docker run --rm -it \
  # Optional .env, can also be provided through CLI
  -v /path/to/your/.env:/app/.env \
  vesu-liquidator \
  --account-address <LIQUIDATOR_ACCOUNT_ADDRESS> \
  --network <NETWORK_NAME> \
  --rpc-url <RPC_URL> \
  --starting-block <BLOCK_NUMBER> \
  --storage-path ./vesu.json # persistent indexer storage into a file
```

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

      --apibara-api-key <APIBARA API KEY>
          Apibara API Key for indexing

  -h, --help
          Print help
```

#### Example: running the bot on Mainnet

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

  🤖 Liquidator 👉 0x42...6c
  🎯 On Mainnet
  🥡 Starting from block 668886


2024-08-23T05:29:06.808482Z  INFO 🧩 Starting the indexer service...
2024-08-23T05:29:06.808583Z  INFO ⏳ Waiting a few moment for the indexer to fetch positions...

2024-08-23T05:29:07.916084Z  INFO [🔍 Indexer] Found new position 0x156fa1e95830c415
2024-08-23T05:29:16.809509Z  INFO 🧩 Starting the oracle service...
2024-08-23T05:29:16.833518Z  INFO 🧩 Starting the monitoring service...

2024-08-23T05:29:16.833561Z  INFO [🔮 Oracle] Fetching latest prices...
2024-08-23T05:29:16.833667Z  INFO [🔭 Monitoring] Checking if any position is liquidable...
2024-08-23T05:29:20.176390Z  INFO [🔮 Oracle] ✅ Fetched all new prices
2024-08-23T05:29:20.177651Z  INFO [🔭 Monitoring] 🤨 They're good.. for now...

#...
```

## Project assistance

If you want to say **thank you** or/and support:

- Add a [GitHub Star](https://github.com/astraly-labs/Vesu-liquidator) to the project.
- Tweet about it.
- Write interesting articles about the project on [Dev.to](https://dev.to/), [Medium](https://medium.com/) or your personal blog.

## Contributing

First off, thanks for taking the time to contribute! Contributions are what make the open-source community such an amazing place to learn, inspire, and create. Any contributions you make will benefit everybody else and are **greatly appreciated**.

Please read [our contribution guidelines](docs/CONTRIBUTING.md), and thank you for being involved!

## Security

We follows good practices of security, but 100% security cannot be assured.
The bot is provided **"as is"** without any **warranty**. Use at your own risk.

_For more information and to report security issues, please refer to our [security documentation](docs/SECURITY.md)._

## License

This project is licensed under the **MIT license**.

See [LICENSE](LICENSE) for more information.
