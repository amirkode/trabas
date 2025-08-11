#!/bin/bash

# init configs
/install/bin/trabas server set-config --gen-key

# start trabas server
/install/bin/trabas server run --host 0.0.0.0 --public-port 8787 --client-port 8789 --cache-client-id --tls
