#!/bin/bash

ORIG_CLK="4.2GHz" #ORIG_CLK="4.2GHz"
TGT_CLK="800MHz"  #TGT_CLK="1.2GHz"

sudo docker stop psql_server

echo "Setting freq to $TGT_CLK"
sudo cpupower --cpu all frequency-set -f $TGT_CLK

echo 3 | sudo tee /proc/sys/vm/drop_caches

sudo docker start psql_server

echo "Waiting for server start..."
sleep 5

start=`date +%s.%N`

cargo run --release

finish=`date +%s.%N`

diff=$(echo "$finish - $start" | bc -l)

echo "Resetting freq to $ORIG_CLK"
sudo cpupower --cpu all frequency-set -f $ORIG_CLK

echo "TPC-H done. elapsed: $diff sec."
