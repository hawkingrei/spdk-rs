# spdk-rs

## Building and Running

```
# First need to build spdk
$ cd /tmp
$ git clone git@github.com:spdk/spdk.git

$ cd /tmp/spdk
$ git checkout v18.07.1
$ git submodule update --init
$ sudo ./scripts/pkgdep.sh

$ ./configure
$ sudo make install
$ ./scripts/setup.sh

# Used for aio backed testing
$ dd if=/dev/zero of=/tmp/aiofile bs=2048 count=5000

$ sudo ldconfig /usr/local/lib

# Need to run dpdk applications as root :(
$ cargo build && sudo target/debug/hello_world
```
