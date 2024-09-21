#!/bin/bash

dirpath=`dirname $0`
cd $dirpath

DEVICE=$(lsblk -o NAME,SIZE | awk '$2 == "8G" {print $1}')

if [[ -z "$DEVICE" || ! "$DEVICE" =~ ^nvme ]]; then
    echo "E: Device not found"
    exit $?
fi

DSTFILE="/mnt/data.bin"
OUTDIR="./output"
mkdir -p $OUTDIR

# echo "Filling random data to $DSTFILE..."

# dd if=/dev/urandom iflag=fullblock of=$DSTFILE bs=1G count=4

echo "Clearing page cache..."

echo 3 | sudo tee /proc/sys/vm/drop_caches

echo "Running 4K random read workload..."

fio randread.fio \
    --filename=$DSTFILE \
    --output="$OUTDIR/baseline.log"