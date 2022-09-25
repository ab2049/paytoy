# Paytoy
Simple example payments engine.

## Assumptions

* Invalid input should cause the program to terminate with no new client balances output

* Transaction amount limit to 4 decimal places is strict. Further digits will be treated as invalid input

* Transaction amounts cannot be negative, negative amounts will be treated as invalid input

* Transaction amounts are not expected for dispute, resolve, chargeback. It present they will be treated as invalid input

* Transaction amounts are expected for deposit or withdrawal. It not present will be treated as invalid input

* Transaction amounts cannot begin with a decimal point. e.g. .1 will be treated as invalid input 

* Extra transaction file columns are invalid input

* Deposits and withdrawals of zero amounts are invalid input

