cyclo
=====

* computes the cyclomatic complexity given a file or directory of files and
visualizes it
* useful for estimating the code complexity of files at a glance
* supports C and C++ file parsing (for now)
* plotly.js is used for the treemap visualization in browser

![](images/xnu-iokit-treemap.png)

treemap
-------

The size of the box corresponds to the number of lines of code, and the hotness of
the color corresponds to the mean function cyclomatic complexity.

The colorscheme can be changed by editing the `colorscale` value in the `cyclo.js`
file in the `webserver/web/scripts` directory. Valid choices are mentioned in the
[Plotly documentation](https://plotly.com/javascript/reference/treemap/#treemap-marker-colorscale)

caveats
-------

The way the mean function cyclomatic is measured is very hacky. It searches for certain keywords when determing decision statements (if, for, while, etc), logical operations (AND, OR), and function definitions. For C/C++ especially, since the function counter actually counts the number of `return` expressions which is pretty bad but there aren't really any other options; AST generation was an absolute pain in C/C++ because of preprocessor defines. The cyclomatic complexity is decently accurate but definitely should be taken with a grain of salt.

usage
-----

The `webserver` and `cyclo` are different packages, so the cyclomatic complexity can be recomputed without having to kill the webserver.


```sh
# build and run cyclo
cd cyclo
cargo build --release
# compute the complexities for the files in the directory ../test
# this will generate a JSON
./target/release/cyclo --path ../test
# or whatever path
# ./target/debug/cyclo --path ../../xnu/iokit
```

```sh
# build and run webserver
cd webserver
cargo build --release
./target/release/webserver --port 3030
```

Also debug info can be printed to a file to check the number of lines of code and cyclomatic complexity.

```sh
cd cyclo
./target/release/cyclo --path ../test --debug
```

Additionally, cargo generates docs super easily. very cool.

```sh
cd cyclo
cargo doc -p cyclo --no-deps
open target/doc/cyclo/index.html
```

to do
-----

* js and py support
