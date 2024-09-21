#!/bin/bash

source .env

read -p "This operation clears $PSQL_DIR. Continue?"

sudo rm -rf $PSQL_DIR
sudo mkdir -p $PSQL_DIR

sudo docker stop psql_server
sudo docker rm psql_server
sudo docker run \
    --name psql_server \
    -e POSTGRES_PASSWORD=$PSQL_PASS \
    -p 5432:5432 \
    -v $PSQL_DIR:/var/lib/postgresql/data \
    --shm-size=256MB \
    --cpus=1 \
    --cpuset-cpus=0 \
    --memory=4g \
    --cpuset-mems=0 \
    -d \
    postgres:${PG_MAJOR}-alpine \
    -c track_io_timing=on \
    -c debug_io_direct=data
