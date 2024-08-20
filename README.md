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
