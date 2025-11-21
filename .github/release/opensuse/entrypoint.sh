#!/bin/bash

SCRIPT_NAME=$(echo $0 | sed -e 's,^.*/,,g' -e 's,\.sh,,g')

echo "$SCRIPT_NAME: starting Slurm services..."

dbus-launch >/dev/null

sudo mkdir -p /run/munge
sudo chmod 0755 /run/munge
sudo chown munge:munge /run/munge

sudo -u munge munged

echo "$SCRIPT_NAME: ready to roll"
exec "$@"

