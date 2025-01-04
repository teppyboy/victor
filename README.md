# Victor

A dead simple temporary mail service (that is not working at all)

## Features

+ [x] Receive mails (with attachment support)
+ [ ] Database for each email address
+ [ ] Backend API
+ [ ] Frontend (will be using Yuuki's one)
... and probably more to implement but okay

## Installation

You'll have to use `bash` for the scripts itself, on Windows both Git Bash and MSYS2 bash are supported (NOT WSL2 BASH SINCE THAT IS LINUX)

1. Clone the repository
2. `cd victor`
3. `./setup.sh`

## Building

`just` is used to simplify the build process, execute `just build` for building debug version, while `just run` for building & running the
debug build.

## License

[MIT](./LICENSE)
