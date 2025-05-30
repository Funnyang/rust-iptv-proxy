### Usage
```
Usage: iptv [OPTIONS]

Options:
  -b, --bind <BIND>                      Bind address:port [default: 127.0.0.1:7878]
  -I, --interface <INTERFACE>            Interface to request
      --udp-proxy                        Use UDP proxy
      --rtsp-proxy                       Use rtsp proxy
  -h, --help                             Print help
```

### Example init.d

```sh
#!/bin/sh /etc/rc.common

START=99
STOP=99

MAC=
USER=
PASSWD=
INTERFACE=pppoe-iptv
BIND=0.0.0.0:7878

start() {
        ( RUST_LOG=info /usr/bin/iptv -u $USER -p $PASSWD -m $MAC -b $BIND -I $INTERFACE --udp-proxy --rtsp-proxy 2>&1 & echo $! >&3 ) 3>/var/run/iptv.pid | logger -t "iptv-proxy" &
}

stop() {
        if [ -f /var/run/iptv.pid ]; then
                kill -9 $(cat /var/run/iptv.pid) 2>/dev/null
                rm -f /var/run/iptv.pid
        fi
}
```

### Build for openwrt
You don't need to install openwrt sdk for this.
```bash
rustup target add x86_64-unknown-linux-musl
cargo build -r --target x86_64-unknown-linux-musl
```
Append `--features rustls-tls` if need tls support.

To reduce binary size, you need to install openwrt sdk to ${openwrt}, and then build with
```bash
rustup +nightly target add x86_64-unknown-linux-musl
rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
toolchain="$(ls -d ${openwrt}/staging_dir/toolchain-*)"
export RUSTFLAGS="-C target-feature=-crt-static -Zlocation-detail=none -C linker=$(ls ${toolchain}/bin/*-openwrt-linux-gcc)"
cargo +nightly build -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort -r --target x86_64-unknown-linux-musl
```

#### Build with openssl for openwrt
You need to install openwrt sdk to ${openwrt}, and then prepare:
```bash
cd ${openwrt}
./scripts/feeds update
./scripts/feeds install openssl
make V=s -j$(nproc)
```
Then build with
```bash
rustup target add x86_64-unknown-linux-musl
export PKG_CONFIG_SYSROOT_DIR=$(ls -d ${openwrt}/staging_dir/target-*)
export PKG_CONFIG_PATH=$PKG_CONFIG_SYSROOT_DIR/usr/lib/pkgconfig
toolchain="$(ls -d ${openwrt}/staging_dir/toolchain-*)"
export TARGET_CC=$(ls ${toolchain}/bin/*-openwrt-linux-gcc)
export STAGING_DIR=$PKG_CONFIG_SYSROOT_DIR
export RUSTFLAGS="-C target-feature=-crt-static -Zlocation-detail=none -C linker=$(ls ${toolchain}/bin/*-openwrt-linux-gcc)"
cargo build -r --target x86_64-unknown-linux-musl --features tls
```
