# Execute

A replacement of [execute-in-repos](https://github.com/diepfote/golang-tools/tree/master/execute-in-repos) and
[execute-on-files](https://github.com/diepfote/golang-tools/tree/master/execute-on-files).

## Options and Args

TODO: paste `execute --help`

TODO: add/write help

## Usage

```text
# simultaneous grep
$ execute -c <(find ~/Repos/scripts/{,bb} -maxdepth 0)  -- grep -r head
[INFO]: config file: /dev/fd/63
[INFO]: number of tasks: 4
[INFO]: number of repos: 2
--
Exit 0: '/Users/florian.sorko/Repos/scripts/bb'
bin/some-file:execute-on-files -no-header -workers 8 -config <(ls "$temp/1"*) "$exec_f"
bin/other:  head -n 1 "$f" > "$what_to_print"
bin/other:qalc --color=0 < <(echo "$calc to hours") | tail -n +3 | head -n1  >> "$what_to_print"
--
Exit 0: '/Users/florian.sorko/Repos/scripts/'
notify-run-make-for-latex.sh:filename="$(head -n 1 Makefile  | cut -d '=' -f2)".pdf
source-me/common-functions.sh:  head -n1 "$(which pip-chill)"
source-me/common-functions.sh:    rsync -rL --list-only "$1" | grep -v '^d' | sort -k3,4r | head -n 5
source-me/common-functions.sh:    rsync -rL --list-only "$2" | grep -v '^d' | sort -k3,4r | head -n "$1"


# we display stdout and stderr for each task
$ time execute -t 20 --files --timeout 5 --config <(find /tmp/files -type f) -- sh -c 'echo asdf; echo 123 >&2; sleep 1; exit 0'
[INFO]: config file: /dev/fd/63
[INFO]: number of tasks: 20
[INFO]: number of files: 2
--
Exit 0: '/tmp/files/1'
stdout:
asdf
stderr:
123
--
Exit 0: '/tmp/files/2'
stdout:
asdf
stderr:
123

real    0m1.144s
user    0m0.020s
sys     0m0.047s


# we report stderr and stdout even if a task times out
$ time execute -t 20 --files --timeout 1 --config <(find /tmp/files -type f) -- sh -c 'echo asdf; echo 123 >&2; sleep 2; exit 0'
[INFO]: config file: /dev/fd/63
[INFO]: number of tasks: 20
[INFO]: number of files: 2
--
Error: timed out in '/tmp/files/1' after 1s.
stdout:
asdf
stderr:
123
--
Error: timed out in '/tmp/files/2' after 1s.
stdout:
asdf
stderr:
123

real    0m1.061s
user    0m0.015s
sys     0m0.046s


# we report the exit code if the task finishes
$ time execute -t 20 --files --timeout 5 --config <(find /tmp/files -type f) -- sh -c 'echo asdf; echo 123 >&2; sleep 1; exit 1'
[INFO]: config file: /dev/fd/63
[INFO]: number of tasks: 20
[INFO]: number of files: 2
--
Exit 1: '/tmp/files/1'
stdout:
asdf
stderr:
123
--
Exit 1: '/tmp/files/2'
stdout:
asdf
stderr:
123

real    0m1.070s
user    0m0.025s
sys     0m0.047s
```

