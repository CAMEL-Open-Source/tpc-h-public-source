#!/bin/bash

dirpath=`dirname $0`
cd $dirpath

DSTFILE="/root/mnt/data.bin"
OUTDIR="./output"
mkdir -p $OUTDIR

echo "Filling random data to $DSTFILE..."

dd if=/dev/urandom of=$DSTFILE bs=1G count=4

echo "Clearing page cache..."

echo 3 | sudo tee /proc/sys/vm/drop_caches

sleep 5

echo "Running 4K random read workload..."

fio randread.fio \
    --filename=$DSTFILE \
    --output="$OUTDIR/lambda.log"