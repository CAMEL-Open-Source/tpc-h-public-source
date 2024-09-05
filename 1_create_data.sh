#!/bin/bash

source .env

cd dbgen

make
./dbgen -f -s $SCALE_FACTOR

# In power test, we need a single refresh pair
# In throughput test, the refresh stream must execute
# a number of pairs of refresh functions serially
#   - Section 5.3.7.7
NUM_REFRESH_PAIR="$(($NUM_STREAM+1))"
./dbgen -f -U $NUM_REFRESH_PAIR -s $SCALE_FACTOR

mv *.tbl ../tbl/
mv *.tbl.* ../tbl/
mv delete.* ../tbl/

cd ../tbl
python3 ../scripts/process_tbl.py 
python3 ../scripts/process_rf1.py $NUM_REFRESH_PAIR
python3 ../scripts/process_rf2.py $NUM_REFRESH_PAIR

mv *.sql ../qry/