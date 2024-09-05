import glob

def main():
    ftbls = glob.glob('*.tbl')

    for ifname in ftbls:
        print(f"Processing {ifname}...")
        ofname = ifname + '.pgres'
        with open(ofname, 'wt') as ofile:
            with open(ifname, 'rt') as ifile:
                num_cols = None
                for line in ifile:
                    line = line.strip()
                    sline = line.split('|')
                    if num_cols is None:
                        num_cols = len(sline)
                    else:
                        assert num_cols == len(sline)
                    last = sline[-1]
                    last = last.strip()
                    assert len(last) == 0

                    ofile.write(f'{"|".join(sline[:-1])}\n')

if __name__ == '__main__':
    main()