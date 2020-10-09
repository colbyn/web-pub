function errorlog {
    echo "\033[1;31m\033[4m$1\033[0m"
}

mkdir -p output
cd ../../

rm -rf example/music-notes/output && cargo run -- \
    build --root example/music-notes

STATUS=$?

if [ "$STATUS" -ne "0" ]; then
    errorlog "BUILD FAILED:"
fi