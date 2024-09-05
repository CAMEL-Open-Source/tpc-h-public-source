# TPC-H Public source

## Introduction
TPC-H test suite for postgresql written in rust and python.

## Configuration
Please refers to .env file.

## Build
For cross-compilation, please refer to [cross-rs](https://github.com/cross-rs/cross).

## Procedures
### 1. DBGen
```
$ ./1_create_data.sh
```

### 2. QGen
```
$ ./2_create_queries.sh
```

### 3. Load
```
$ ./3_load_db.sh
```
Load time will be reported and is part of TPC-H test.

### 4. Run
```
$ cargo run
```

## Caution
The test is irreversible, and even if the test was interrupted in the middle, it should be resumed from at least script number 3 (3_load_db.sh).

## References
* TPC Benchmark H Standard Specification Revision 3.0.1
* https://www.researchgate.net/publication/236132848_Benchmarking_with_TPC-H_on_Off-the-Shelf_Hardware_An_Experiments_Report