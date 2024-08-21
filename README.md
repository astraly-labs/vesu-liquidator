# Vesu-liquidator

Liquidation bot for Vesu.

## Requirements

### Protobuf

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

### Environment Variables

Create an `.env` file following the example file and fill the keys.

## Usage

See:

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
