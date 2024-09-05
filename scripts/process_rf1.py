from collections import OrderedDict
import sys

RF1_TRX_SIZE = 100

RF1_ORDER = """INSERT INTO ORDERS(
O_ORDERKEY,
O_CUSTKEY,
O_ORDERSTATUS,
O_TOTALPRICE,
O_ORDERDATE,
O_ORDERPRIORITY,
O_CLERK,
O_SHIPPRIORITY,
O_COMMENT) VALUES\n"""

RF1_LITEM="""INSERT INTO LINEITEM(
L_ORDERKEY,
L_PARTKEY,
L_SUPPKEY,
L_LINENUMBER,
L_QUANTITY,
L_EXTENDEDPRICE,
L_DISCOUNT,
L_TAX,
L_RETURNFLAG,
L_LINESTATUS,
L_SHIPDATE,
L_COMMITDATE,
L_RECEIPTDATE,
L_SHIPINSTRUCT,
L_SHIPMODE,
L_COMMENT) VALUES\n"""

def build_rf1_trx(orders, litems):
    trx = "BEGIN;\n"

    trx += RF1_ORDER
    for order in orders:
        v = ",".join(map(lambda v: f"'{v}'", order))
        trx += f"({v}),\n"
    trx = trx[:-2] # Remove trailing comma: Forbidden in SQL
    trx += ';\n'

    trx += RF1_LITEM
    for litem in litems:
        v = ",".join(map(lambda v: f"'{v}'", litem))
        trx += f"({v}),\n"
    trx = trx[:-2] # Remove trailing comma: Forbidden in SQL
    trx += ';\n'
    trx += "COMMIT;\n"

    return trx

def main():
    num_refresh = int(sys.argv[1])

    for i in range(1, num_refresh+1):
        pairs = []
        oids = set()
        with open(f'orders.tbl.u{i}', 'rt') as f_order:
            with open(f'lineitem.tbl.u{i}', 'rt') as f_litem:
                last_litem = None
                for line_order in f_order:
                    line_order = line_order.strip()
                    if len(line_order) == 0:
                        continue
                    sline_order = line_order.split('|')[:9]
                    oid = int(sline_order[0])

                    litems = [last_litem] if last_litem else []
                    last_litem = None
                    for line_litem in f_litem:
                        line_litem = line_litem.strip()
                        if len(line_litem) == 0:
                            continue
                        sline_litem = line_litem.split('|')[:16]
                        loid = int(sline_litem[0])
                        if loid == oid:
                            litems.append(sline_litem)
                        else:
                            assert last_litem is None
                            last_litem = sline_litem
                            break
                    
                    assert oid not in oids
                    oids.add(oid)
                    pairs.append((sline_order, litems))

        lp = len(pairs)
        assert lp > 0
        assert lp % 1500 == 0

        with open(f'rf1.u{i}.sql', 'wt') as of:
            for i in range(0, lp, RF1_TRX_SIZE):
                start_idx = i
                end_idx = min((i + RF1_TRX_SIZE, lp))
                cur_pairs = pairs[start_idx:end_idx]

                cur_orders = []
                cur_litems = []
                for order, litems in cur_pairs:
                    cur_orders.append(order)
                    cur_litems.extend(litems)
                
                trx = build_rf1_trx(cur_orders, cur_litems)

                of.write(trx)
            
if __name__ == '__main__':
    main()