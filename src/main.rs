use anyhow::Result;
use clap::Parser;
use engine::{Account, Transaction};
use std::collections::BTreeMap;
use std::io;
use std::io::{Read, Write};
use std::path::PathBuf;

mod engine;

#[derive(Parser, Debug)]
#[clap(author = "Simon Adameit")]
struct Args {
    transaction_csv: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let input = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(args.transaction_csv)?;
    let output = csv::Writer::from_writer(io::stdout());
    run(input, output)
}

fn run<In, Out>(mut input: csv::Reader<In>, mut output: csv::Writer<Out>) -> Result<()>
where
    In: Read,
    Out: Write,
{
    // We sort the accounts by client id for more predictable output
    let mut accounts = BTreeMap::new();
    for maybe_transaction in input.deserialize() {
        let transaction: Transaction = maybe_transaction?;
        let client = transaction.client;
        let account = accounts.entry(client).or_insert_with(|| Account::new(client));
        account.handle(transaction)?;
    }
    for account in accounts.values() {
        output.serialize(account.info())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;

    #[test]
    fn withdraw_and_deposit() {
        assert_result(
            "\
type,    client,  tx,  amount
deposit,      1,   1,     1.0
deposit,      2,   2,     2.0
deposit,      1,   3,     2.0
withdrawal,   1,   4,     1.5
withdrawal,   2,   5,     3.0",
            "\
client,available,held,total,locked
1,1.5,0,1.5,false
2,2,0,2,false
",
        )
    }

    #[test]
    fn dispute() {
        assert_result(
            "\
type,    client,  tx,  amount
deposit,      1,   1,     1.0
dispute,      1,   1
",
            "\
client,available,held,total,locked
1,0,1,1,false
",
        )
    }

    #[test]
    fn dispute_and_resolve() {
        assert_result(
            "\
type,    client,  tx,  amount
deposit,      1,   1,     1.0
dispute,      1,   1
resolve,      1,   1
",
            "\
client,available,held,total,locked
1,1,0,1,false
",
        )
    }

    #[test]
    fn dispute_and_chargeback() {
        assert_result(
            "\
type,    client,  tx,  amount
deposit,      1,   1,     1.0
dispute,      1,   1
chargeback,   1,   1
",
            "\
client,available,held,total,locked
1,0,0,0,true
",
        )
    }

    #[test]
    fn double_dispute() {
        assert_result(
            "\
type,    client,  tx,  amount
deposit,      1,   1,     1.0
deposit,      1,   2,     3.0
dispute,      1,   1
dispute,      1,   1
",
            "\
client,available,held,total,locked
1,3,1,4,false
",
        )
    }

    #[test]
    fn double_chargeback() {
        assert_result(
            "\
type,    client,  tx,  amount
deposit,      1,   1,     1.0
deposit,      1,   2,     3.0
dispute,      1,   1
chargeback,   1,   1
chargeback,   1,   1
",
            "\
client,available,held,total,locked
1,3,0,3,true
",
        )
    }

    #[test]
    fn double_dispute_and_chargeback() {
        assert_result(
            "\
type,    client,  tx,  amount
deposit,      1,   1,     1.0
deposit,      1,   2,     3.0
dispute,      1,   1
chargeback,   1,   1
dispute,      1,   1
chargeback,   1,   1
",
            "\
client,available,held,total,locked
1,3,0,3,true
",
        )
    }

    fn assert_result(input: &'static str, output: &'static str) {
        let mut bytes = Vec::new();
        let reader = csv::ReaderBuilder::new()
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(input.as_bytes());
        let writer = csv::Writer::from_writer(&mut bytes);
        run(reader, writer).unwrap();
        assert_eq!(str::from_utf8(&bytes).unwrap(), output);
    }
}
