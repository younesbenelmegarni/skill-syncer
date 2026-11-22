from pathlib import Path
import os, hashlib

def snapshot(root):
    result = {}
    if not root.exists():
        return result
    def visit(p):
        st = p.lstat()
        entry = [st.st_mode, st.st_size, st.st_mtime_ns, st.st_ctime_ns]
        if p.is_symlink():
            entry.append(os.readlink(p))
        elif p.is_file():
            entry.append(hashlib.sha256(p.read_bytes()).hexdigest())
        result[str(p)] = entry
        if p.is_dir() and not p.is_symlink():
            for child in sorted(p.iterdir()):
                visit(child)
    visit(root)
    return result
