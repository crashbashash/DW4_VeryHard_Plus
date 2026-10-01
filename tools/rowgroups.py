#!/usr/bin/env python3
"""Print the row->type groups for the Brutal preset (spec 12.1).

The disc's three patchable columns are numbers, so the type a row belongs to
exists only in the runtime MODEL column. This reads a RAM snapshot of the
authored table and prints the groups. The offsets are the ones patch_iso.py
uses; tests/rowgroup_snapshot.rs in dw4vhp-core is a Rust port that asserts the
committed ROW_GROUPS matches this derivation.

  python3 tools/rowgroups.py --markdown SNAPSHOT
  python3 tools/rowgroups.py --rust SNAPSHOT

No game data is written; only row indices and model names are printed.
"""
import argparse, collections, struct

DB, ROWS = 0x1748840, 649

def load(path):
    data = open(path, 'rb').read()
    u32 = lambda a: struct.unpack_from('<I', data, a)[0]
    hp = [struct.unpack_from('<i', data, u32(u32(DB + 0x1C) + 0x10) + 4 * r)[0]
          for r in range(ROWS)]

    def model_of(r):
        table = u32(u32(DB + 0x34) + 0x10)
        entry = u32(table + 4 * r)
        if not entry:
            return None
        ptr = u32(entry + 8)
        if not (0x100000 < ptr < 0x2000000):
            return None
        end = data.find(b'\0', ptr, ptr + 64)
        name = data[ptr:end].decode('latin1')
        return name if name and all(32 <= ord(c) < 127 for c in name) else None

    groups = collections.OrderedDict()
    for r in range(ROWS):
        name = model_of(r)
        if name:
            groups.setdefault(name, []).append(r)
    return groups, hp

def ranges(rows):
    out, start, prev = [], rows[0], rows[0]
    for r in rows[1:]:
        if r == prev + 1:
            prev = r
            continue
        out.append(str(start) if start == prev else f'{start}-{prev}')
        start = prev = r
    out.append(str(start) if start == prev else f'{start}-{prev}')
    return ', '.join(out)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('snapshot')
    ap.add_argument('--markdown', action='store_true')
    ap.add_argument('--rust', action='store_true')
    args = ap.parse_args()
    groups, hp = load(args.snapshot)
    if args.markdown:
        print('| model | rows | top |')
        print('|---|---|---|')
        for name, rows in groups.items():
            print(f'| `{name}` | {ranges(rows)} | {max(rows, key=lambda r: hp[r])} |')
        return
    print('pub const ROW_GROUPS: &[RowGroup] = &[')
    for name, rows in groups.items():
        top = max(rows, key=lambda r: hp[r])
        body = ', '.join(str(r) for r in rows)
        print(f'    RowGroup {{ model: "{name}", top: {top}, rows: &[{body}] }},')
    print('];')

if __name__ == '__main__':
    main()
