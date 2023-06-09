# streamfly

Streamfly aims to be a stream-oriented Pub/Sub framework.

Unlike traditional Pub/Sub systems, instead of transffering data
packets(messages), streamfly focuses on transffering streams, which means users
could publish or subscrbie a stream(Reader & Writer). Then developers can
manipulate these streams in their applications, just like the stdin & stdout.

## Build

```sh
cargo build
```

```sh
cargo build --examples
```

## Run

```sh
./target/debug/streamfly serve
```

```sh
./target/debug/examples/sub
```

```sh
./target/debug/examples/pub
```
