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
#   OVH_SFTP_USER        <main-login>-<suffix>, an SFTP-only user; one account
#                        serves every environment
#   OVH_SFTP_PASSWORD    masked, protected
#   OVH_SSH_KNOWN_HOSTS  path to a known_hosts file (a GitLab "File" variable)
#                        pinning the cluster's host key; no TOFU
#   OVH_SFTP_DIR         REQUIRED, the environment discriminant: target
#                        directory relative to the account's root folder, e.g.
#                        dev-voc/voc (dev) or www/voc (prd). The shared account
#                        can reach WordPress, so a missing or sloppy value is
#                        refused rather than defaulted: `mirror` into the
#                        wrong directory deletes whatever is there.
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
case "/$dir/" in
  //|/./|*/../*|*/./*|//*) echo "OVH_SFTP_DIR='$dir' must be a relative path below the account root, without . or .. segments" >&2; exit 2 ;;
esac

# The password reaches lftp through its environment, never its argv.
export LFTP_PASSWORD="$OVH_SFTP_PASSWORD"
lftp --env-password -u "$OVH_SFTP_USER" -p "$port" "sftp://$OVH_SFTP_HOST" <<LFTP
set sftp:connect-program "ssh -a -x -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$OVH_SSH_KNOWN_HOSTS"
set net:max-retries 3
set net:timeout 30
mirror --reverse --verbose --no-perms $delete "$tree" "$dir"
bye
LFTP
