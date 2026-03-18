cd board
cargo build
cd target/thumbv7m-none-eabi/debug
~/toolchains/gcc-arm-embedded/bin/arm-none-eabi-objcopy -O binary app app.bin
ls -lah app*
cd ../../../
cd ../
