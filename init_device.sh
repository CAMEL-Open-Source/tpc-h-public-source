#!/bin/bash

DEVICE=$(lsblk -o NAME,SIZE | awk '$2 == "8G" {print $1}')

if [[ -z "$DEVICE" || ! "$DEVICE" =~ ^nvme ]]; then
    echo "E: Device not found"
    exit $?
fi

sudo mkfs.ext4 /dev/$DEVICE
sudo mount /dev/$DEVICE -o sync /mnt
