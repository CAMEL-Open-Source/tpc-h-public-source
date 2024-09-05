#!/bin/bash

source .env

export PGPASSWORD=$PSQL_PASS

read -p "This operation clears existing database [$PSQL_DB]. Continue?"

# Drop existing database
psql -h$PSQL_HOST -U$PSQL_USER -d postgres -c "DROP DATABASE IF EXISTS $PSQL_DB;"

psql -h$PSQL_HOST -U$PSQL_USER -d postgres -c "ALTER SYSTEM SET jit = off;"
psql -h$PSQL_HOST -U$PSQL_USER -d postgres -c "ALTER SYSTEM SET track_io_timing = off;"

start=`date +%s.%N`

# Create fresh database
echo "---Create database..."
psql -h$PSQL_HOST -U$PSQL_USER -d postgres -c "CREATE DATABASE $PSQL_DB;"

# Create tables
echo "---Populating tables..."
psql -h$PSQL_HOST -U$PSQL_USER -d $PSQL_DB < dss/tpch-load.sql

echo "---Creating primary keys..."
psql -h$PSQL_HOST -U$PSQL_USER -d $PSQL_DB < dss/tpch-pkeys.sql

echo "---Creating foreign keys..."
psql -h$PSQL_HOST -U$PSQL_USER -d $PSQL_DB < dss/tpch-alter.sql

echo "---Creating indexes..."
psql -h$PSQL_HOST -U$PSQL_USER -d $PSQL_DB < dss/tpch-index.sql

echo "---Checkpointing..."
psql -h$PSQL_HOST -U$PSQL_USER -d $PSQL_DB < dss/tpch-chkpt.sql

finish=`date +%s.%N`

diff=$(echo "$finish - $start" | bc -l)

echo "---Turning on cumulative statistics..."
psql -h$PSQL_HOST -U$PSQL_USER -d postgres -c "ALTER SYSTEM SET track_io_timing = on;"

echo "Load done. elapsed: $diff sec."