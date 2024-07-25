#!/bin/sh

# Initialize any environment variables or execute commands
export REDIS_HOST=${REDIS_HOST:-127.0.0.1}
export REDIS_PORT=${REDIS_PORT:-6379}
export REDIS_PASS=${REDIS_PASS:-serverpass}

# init configs
/install/bin/trabas server set-config --gen-key --redis-host $REDIS_HOST --redis-port $REDIS_PORT --redis-pass $REDIS_PASS

# start redis
redis-server --requirepass $REDIS_PASS --daemonize yes

# start trabas server
/install/bin/trabas server run --host 0.0.0.0 --public_port 8787 --client_port 8789 
