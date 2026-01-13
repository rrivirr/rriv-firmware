cd board/app
cargo clean
cargo build --release
cd ../../

probe-rs download board/target/thumbv7m-none-eabi/release/app \
        --chip STM32F103RE  \
        --protocol swd \
        --allow-erase-all \
        --chip-erase

probe-rs reset \
        --chip STM32F103RE  \
        --protocol swd \

