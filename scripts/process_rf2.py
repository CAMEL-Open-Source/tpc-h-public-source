import sys

RF2_TRX_SIZE = 100
RF2_DELETE="""DELETE FROM ORDERS WHERE O_ORDERKEY IN"""

def build_rf2_trx(oids):
    trx = "BEGIN;\n"

    trx += RF2_DELETE
    trx += "("
    trx += ",".join(map(str, oids))
    trx += ');\n'
    trx += "COMMIT;\n"

    return trx

def main():
    num_refresh = int(sys.argv[1])

    for i in range(1, num_refresh+1):
        oids = []
        oids_dict = {}
        with open(f'delete.{i}', 'rt') as f_dels:
            for line in f_dels:
                line = line.strip()
                if len(line) == 0:
                    continue
                sline = line.split('|')
                assert len(sline) == 2
                oid = int(sline[0])
                assert oid not in oids_dict
                oids.append(oid)

        lp = len(oids)
        assert lp > 0
        assert lp % 1500 == 0

        with open(f'rf2.u{i}.sql', 'wt') as of:
            for i in range(0, lp, RF2_TRX_SIZE):
                start_idx = i
                end_idx = min((i + RF2_TRX_SIZE, lp))
                cur_oids = oids[start_idx:end_idx]
                trx = build_rf2_trx(cur_oids)

                of.write(trx)
            
if __name__ == '__main__':
    main()