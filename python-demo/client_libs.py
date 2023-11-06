import os
import sys

try:
    if os.path.exists('../bindings_python/client.py'):
        sys.path.append('../bindings_python/')
        ext = sys.platform != 'darwin' and '.so' or '.dylib'

        def whois_latest() -> str:
            mtime = None
            last_so = None
            for p in ['../target/debug/', '../target/release/']:
                f = os.path.join(p, 'librestsend_sdk'+ext)
                if not os.path.exists(f):
                    continue
                st = os.stat(f)
                if not mtime or st.st_mtime > mtime:
                    mtime = st.st_mtime
                    last_so = f
            assert last_so is not None

            target_so = '../bindings_python/libuniffi_client'+ext
            if os.path.exists(target_so):
                os.remove(target_so)
            os.link(last_so, target_so)
            return last_so
        whois_latest()
    import client
except ImportError:
    print('Please run `cargo build` first')
    sys.exit(1)


whois_latest()
