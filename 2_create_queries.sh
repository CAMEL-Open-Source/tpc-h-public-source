#!/bin/bash

source .env

cd dbgen
export DSS_QUERY='./queries'

./qgen -s $SCALE_FACTOR > ../qry/query_power.sql

for sid in $(seq 1 $NUM_STREAM)
do
    ../dbgen/qgen -s $SCALE_FACTOR -p $sid > ../qry/query_throughput_$sid.sql
done