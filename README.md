# Victor

A dead simple temporary mail service (that is not usable at all)

## Features

+ [x] Receive mails (tested with GMail)
    > Attachments are broken and currently disabled by default.
+ [x] Database
    > Valkey/Redis are supported as the database server.
+ [ ] API
+ [ ] Frontend

... and probably more to implement (like Spam Filtering, SMTPS,...) but okay

## Installation

You'll have to use `bash` for the scripts itself, on Windows both Git Bash and MSYS2 bash are supported

1. Clone the repository
2. `cd victor`
3. `./setup.sh`
4. Setup the Redis/Valkey database server.

## Building

`just` is used to simplify the build process, execute `just build` for building debug version, while `just run` for building & running the debug build.

## FAQ

### Why Rust?

I hate myself for using Rust in this project to be honest.

### About the name

["Victor Grantz"](https://id5.fandom.com/wiki/Postman) is the name of the playable character "Postman" in [Identity V](https://idv.163.com)

## License

[MIT](./LICENSE)
