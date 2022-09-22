
# A trading engine example in rust

There are thee main tasks that the application needs to solve:
- data ingestion
- transaction handling
- reporting

The transaction handling is independent for each client account,
as transactions only ever concern one client. The business logic
is in the `engine` module, and the io handling and setup are in the 
`main` module. The program doesn't use multi-threading or async.
The separation of concerns allows us to introduce it when the need arises.

## Usage

```
$ cargo run -- --help

trading_engine
Simon Adameit

USAGE:
    trading_engine <TRANSACTION_CSV>

ARGS:
    <TRANSACTION_CSV>

OPTIONS:
    -h, --help    Print help information
```

## Correctness

The transaction and dispute state are enums. That should help
to make the prerequisites for transactions clear. As the specification
suggested, a transaction that fails to meet its prerequisites is
handled as client error and ignored.

## Interpretation of the requirements

I made the following assumptions:
- Withdrawals cannot be disputed
- Negative balances due to disputes are allowed
