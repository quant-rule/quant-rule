#!/bin/sh
# Assert that the runtime image carries no Python interpreter and no .py file.
#
# The Dockerfile runs this in the final stage, so a base-image change or an
# added package fails the build. The release workflow runs it against the built
# image, so the published digest carries the same evidence. The script ships in
# the image for that second run, which is why it names the interpreter it
# rejects.
#
# Plain POSIX sh: the runtime is Debian slim and ships dash, not bash.
set -eu

status=0

for name in python python3; do
    if found="$(command -v "$name" 2>/dev/null)" && [ -n "$found" ]; then
        echo "error: the image carries a Python interpreter: $name at $found" >&2
        status=1
    fi
done

# An interpreter binary can sit on disk without a python/python3 alias.
for directory in /usr/bin /usr/local/bin; do
    [ -d "$directory" ] || continue
    matches="$(find "$directory" -maxdepth 1 -name 'python*' 2>/dev/null || true)"
    if [ -n "$matches" ]; then
        echo "error: the image carries a Python interpreter binary:" >&2
        printf '%s\n' "$matches" | while IFS= read -r file; do
            printf '  %s\n' "$file" >&2
        done
        status=1
    fi
done

# The runtime paths hold what the Dockerfile copies plus the distribution. A
# module tree under /usr is the usual way an interpreter arrives.
for path in /usr /etc /opt /srv /app /root /home; do
    [ -d "$path" ] || continue
    matches="$(find "$path" -type f -name '*.py' 2>/dev/null || true)"
    if [ -n "$matches" ]; then
        echo "error: the image carries Python files:" >&2
        printf '%s\n' "$matches" | while IFS= read -r file; do
            printf '  %s\n' "$file" >&2
        done
        status=1
    fi
done

# The build context excludes the repository tooling; assert nothing copied it
# back in under the conventional path.
if [ -e /tools ]; then
    echo "error: the image carries a repository tools directory: /tools" >&2
    status=1
fi

if [ "$status" -ne 0 ]; then
    exit 1
fi
echo "image python gate ok: no interpreter and no .py file"
