Each time you make a change in the models of the database, you can apply the migration with the following command:
```
 cargo run -p migration --bin reset_database
 ```
 It destroys and create again all tables. Every data are erased.


The command to build, run tests and format the code is:
```
cargo fmt && cargo build && cargo test
```