set -e

watchexec --exts md,html --ignore output/ -- './scripts/build.sh && echo "\n==reloaded==\n"'