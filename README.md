# Paytoy
Simple example payments engine. Takes a file of transactions and outputs client balances taking into account deposits, withdrawals, disputes, resolutions and chargebacks.

## Assumptions

* Invalid input should cause the program to terminate with no new client balances output

* Transaction amount limit to 4 decimal places is strict. Further digits will be treated as invalid input

* The underlying rust_decimal library will error if it overflows for transactions or balances.  If due to hyper inflation more digits are needed consider using bigdecimal or other arbitary precision crate

* Transaction amounts cannot be negative, negative amounts will be treated as invalid input

* Transaction amounts are not expected for dispute, resolve, chargeback. It present they will be treated as invalid input

* Transaction amounts are expected for deposit or withdrawal. It not present will be treated as invalid input

* Transaction amounts cannot begin with a decimal point. e.g. .1 will be treated as invalid input 

* Extra transaction file columns are invalid input

* Deposits and withdrawals of zero amounts are invalid input

* Duplicate transaction ids for deposits or withdrawals are invalid input

* Unknown transaction ids for dispute, resolve, chargebacks are errors from the payment partner and will be ignored

## Design choices
Although this toy reads from a simple CSV file, its designed with tokio tasks sharded by mod of client id as an example of how one might structure if was running for real and reading from multiple input streams and then dispatching to sharded client processing.

Using integer math for precision as binary floating point can't represent numbers like 0.0001 exactly. 

Each shard handles multiple clients and can use regular unlocked maps as no other task is handling that shard of clients.

For simplicity using anyhow::Error and bail!. In this was a real payment library would likely use thiserror::Error instead.

Using storage of transactions that could be reverse in memory for simplicity vs attempting something like LevelDB.

## Safety and Robustness

Check the dependencies for known vulns with cargo-audit.  None at time of writing

Errors are checked.  Errors cause the program to exit without outputing new client balances

## Efficiency

Valid Transactions have no deadline for reversal, and thus need to stored unaggregated in the `balance::Balance::trans` `balance::TranRecord`s for the duration of the run. Each on has have approximate size of `(hash_map::Entry<ids::TxId, balance::TranRecord>)` which is around 32 bytes on x64_64 linux and current rust stable.  Given max 2^32 transactions, this means lower bound when run with max possible transactions for on memory usage will be 128GiB.

Invalid Transactions should take no TranRecord storage, although they may take up space in io buffers and queues.

If insufficient RAM is present but enough Swap is present then performance should be similar to an explicily memmap'd approach.  

In a real system one may have a larger TransactionId and use something like sharded LevelDB or a distributed store to keep per process size under control.

In a real system with a clock and transaction timestampds, if some clients or payment partners had a time limit on reversal then the solution could be made more efficient by pruning stored state once clock advances past the deadline(s) for retention for a balance.

## Maintainability

Automated tests,  easy to add new test cases if a regression is found.

Uses the type system (e.g. newtypes, enums) to detect problems at compile time and reduce possible coding errors by maintainers.

Single threaded form is simpler, pretty easy to remove tokio changes if desired as they are contained to main