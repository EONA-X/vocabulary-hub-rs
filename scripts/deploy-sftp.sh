#!/usr/bin/env sh
# Uploads <tree> into OVH_SFTP_DIR on an OVH web hosting SFTP account.
#
#   scripts/deploy-sftp.sh <tree> mirror     remote becomes exactly <tree>
#                                            (dev hosts: deletions included)
#   scripts/deploy-sftp.sh <tree> additive   upload new and changed files,
#                                            delete nothing (eona-x.eu: a
#                                            version once published stays)
#
# Environment (GitLab CI variables scoped to the deploy environment):
#   OVH_SFTP_HOST        ssh.clusterNNN.hosting.ovh.net
#   OVH_SFTP_USER        <main-login>-<suffix>, an SFTP-only user whose root
#                        folder is the environment's web root (dev: dev-voc/)
#   OVH_SFTP_PASSWORD    masked, protected
#   OVH_SSH_KNOWN_HOSTS  path to a known_hosts file (a GitLab "File" variable)
#                        pinning the cluster's host key; no TOFU
#   OVH_SFTP_DIR         REQUIRED target directory, relative to the account's
#                        root folder, e.g. voc. A missing or sloppy value is
#                        refused rather than defaulted: `mirror` into the wrong
#                        directory deletes whatever is there. The root itself
#                        (`.`) is accepted only for `additive`, which never
#                        deletes (the dev root files).
set -eu
tree=$1; mode=$2
case "$mode" in
  mirror) delete=--delete ;;
  additive) delete= ;;
  *) echo "mode must be mirror or additive, not '$mode'" >&2; exit 2 ;;
esac
: "${OVH_SFTP_HOST:?}" "${OVH_SFTP_USER:?}" "${OVH_SFTP_PASSWORD:?}" "${OVH_SSH_KNOWN_HOSTS:?}"
port=${OVH_SFTP_PORT:-22}
dir=${OVH_SFTP_DIR:-}
if [ "$dir" = . ] && [ "$mode" = additive ]; then
  : # the account root, add-only
else
  case "/$dir/" in
    //|/./|*/../*|*/./*|//*) echo "OVH_SFTP_DIR='$dir' must be a relative path below the account root, without . or .. segments (the root itself only with additive)" >&2; exit 2 ;;
  esac
fi

# The password reaches lftp through its environment, never its argv.
export LFTP_PASSWORD="$OVH_SFTP_PASSWORD"
lftp --env-password -u "$OVH_SFTP_USER" -p "$port" "sftp://$OVH_SFTP_HOST" <<LFTP
set sftp:connect-program "ssh -a -x -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$OVH_SSH_KNOWN_HOSTS"
set net:max-retries 3
set net:timeout 30
mirror --reverse --verbose --no-perms $delete "$tree" "$dir"
bye
LFTP
