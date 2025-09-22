_: 
  @just -l

build:
  RUSTFLAGS='-C link-arg=-lc++_shared -C link-arg=-landroid' ~/.cargo/bin/dx build --platform android --verbose --trace

serve:
  RUSTFLAGS='-C link-arg=-lc++_shared -C link-arg=-landroid' ~/.cargo/bin/dx serve --platform android

